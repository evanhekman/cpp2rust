# Copyright (c) Microsoft Corporation. #
# Licensed under the MIT license.      #

import os
from pathlib import Path

from infer import LLM
from veval import EvalScore, VEval

from utils import clean_code, code_change_is_safe, evaluate, extract_verus_errors


class Generation:
    def __init__(self, config, logger, examples=None,):
        self.config = config
        self.llm = LLM(config, logger)
        self.logger = logger
        self.examples = examples

        self.logger.info("Generation initialized with examples: %s", self.examples)

    # This long prompt is used in the alternative design where proof generation is done in one shot
    # without further phases of refinement or repair
    def direct_inference(
        self,
        code,
        verus_output="",
        temp=0,
        error="",
    ):
        system = "You are an experienced formal language programmer. You are very familiar with Verus, which is a tool for verifying the correctness of code written in Rust."

        instruction = """
## Step 1: Add Loop Invariants
Your mission is to add loop invariants to the given Rust code, if there are loops in the code, so that Verus can verify the give function behaves exact what is described in the specifications.

Here are some principles that you have to follow:
Respond with Rust code only, do not include any explanation.
You should never change or delete the requires and ensures, or change the implementation itself. Feel free to change/delete loop invariants, decreases clauses, and other ghost stuff.
When adding loop invariants in Verus, remember: 
- If an upper bound or a lower bound about a constant function parameter (e.g., X < ..., X > ...) is provided in the function pre-condition (i.e., in the `requires' code block at the beginning of the function),
please copy that (e.g., X < 10, X > 5) as a loop invariant to every loop in the function.
- The invariant must hold *at the start of the loop* and *after each iteration*, including the last one. 
- Therefore, bounds like i <= a@.len() + 1 are required if i is incremented +2 or greater.
- Follow the examples to learn about syntax.
"""
        if verus_output != "":
            instruction += "Here is the output Verus had on this code: ";
            instruction += verus_output

        examples = []

        for f in sorted(os.listdir(os.path.join(self.config.example_path, "input"))):
            if f.endswith(".rs") and f[2] in self.examples:
                input_file = os.path.join(self.config.example_path, "input", f)
                output_file = os.path.join(self.config.example_path, "output", f)
                input_content = open(input_file).read()
                output_content = open(output_file).read()
                examples.append({"query": input_content, "answer": output_content})

        with open("example.log", "w") as f:
            for ex in examples:
                f.write(ex["query"] + "\n")
                f.write(ex["answer"] + "\n\n")

        self.logger.info("Direct Inference ...")
        return self.llm.infer_llm(
            self.config.aoai_generation_model,
            instruction,
            examples,
            code,
            system,
            max_tokens=self.config.max_token,
            temp=temp,
        )

    def generate_with_proof_func(
        self,
        code,
        verbose=False,
        temp=1.0,
        temp_dir=Path("output-intermediate-temp"),
    ):
        """
        Generate the proof code with the whole pipeline.
        This is the default pipeline for proof generation in AutoVerus.
        """
        temp_dir.mkdir(parents=True, exist_ok=True)
        original_code = code

        best_score_of_all = EvalScore.get_worst_score()

        attempt = 0

        output = ""
        while attempt < 40:
            # print(original_code)
            self.logger.info(f"Direct inference attempt {attempt}")
            # Now use direct_inference.
            
            code = self.direct_inference(original_code, temp=temp, verus_output=output)
            code = code[0].removeprefix('```rust\n').removesuffix('```')
            self.logger.info(f"Checking candidate {attempt}")
            cand_code = clean_code(code)

            veval = VEval(cand_code, self.logger)
            score = veval.eval_and_get_score()

            is_safe_code_change = code_change_is_safe(
                original_code, cand_code, self.config.verus_path, self.logger
            )

            out_code = cand_code + "\n// Score: " + str(score) + "\n// Safe: " + str(is_safe_code_change)
            (temp_dir / f"{attempt}.rs").write_text(
                out_code
            )

            if score.is_correct() and is_safe_code_change:
                self.logger.info("Verus succeeded!!")
                return cand_code

            # save verus output for next iteration
            output = extract_verus_errors(veval.rustc_out)
            # update code to be generated code
            original_code = cand_code
            print(output)
            # if unsafe code was generated or if no valid code is fine at all,
            # better try another invocation to get more candidates
            self.logger.info("Regenerate...")
            attempt += 1

        self.logger.info("Max attempts reached, giving up")
        return cand_code

    def run(self, input_file, output_file, args: dict = None):
        if args is None:
            args = {}
        temp = args.get("temp", 1.0)

        content = open(input_file).read()
        output_file = Path(output_file)
        output_dir = output_file.parent
        output_dir.mkdir(parents=True, exist_ok=True)
        temp_dir = Path(output_dir) / ("intermediate-" + output_file.stem)
        temp_dir.mkdir(parents=True, exist_ok=True)

        self.logger.info("Generating proof code")
        self.logger.info("Temperature: " + str(temp))

        # default/recommended
        code = self.generate_with_proof_func(
            content,
            verbose=True,
            temp=temp,
            temp_dir=temp_dir,
        )

        score, _ = evaluate(code, self.config.verus_path)
        is_safe = code_change_is_safe(
            content, code, self.config.verus_path, self.logger, debug=False
        )
        code += "\n// Score: " + str(score)
        code += "\n// Safe: " + str(is_safe)

        with open(output_file, "w") as wf:
            wf.write(code)
        self.logger.info("finished!")
