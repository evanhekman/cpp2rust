### overview
- these are guidelines for the synthesizer/ portion of the repo
- the synthesizer works generally as follows:
  - inputs: cpp/ files, prepost/ files, and processed/ files
  - output: stitched/ files
  - the cpp/ files are only used to generate test cases (by running the actual cpp)
  - the prepost/ files are only used as a wrapper around the synthesized rust code
  - the processed/ files should contain all critical information to guide heuristics
- see root level README.md for general context about the repository

### guidelines
- keep files below 800 lines
- keep functions simple
- functions should only do one thing
- errors should never be swept under the rug
- follow best design practices (as always)
- never (ever) create markdown files. no, really, not ever.
- perform git operations if and only if they are readonly. do not ever push or commit to git.

### testing
- prioritize quick tests that give immediate information over long tests that are more like black boxes
- if failing on a long test, consider why:
  - if the test exhausts all possible options before the time limit, *correct solution(s) are being pruned*
  - if the test fails due to time limit, *correct solution(s) may be reachable, but the heuristics are too slow*
  - if this ^ happens, instead of increasing the time limit, isolate the relevant portion of the test case and shrink it down to a new test case. then, try to meaningfully speed up that test case before returning to the main test case.
- you should never need a timeout of more than 30 seconds
- if you find yourself running a command more than once, consider whether it should be incorporated into the `justfile`
- use `just synthesize <benchmark> [target]` to run synthesis; omitting target runs all. builds automatically.

### interaction
always alert the user when you
- create a new test case
- find a key bug
- find convincing results for the effectiveness/ineffectiveness of a heuristic
- make changes to the `justfile`
- if you need to do any of these things:
  - create a python interpreter
    - run a python script that is NOT via an established shebang
  - cross-reference your results
  - install a new dependency
- just ask the user

### project structure
cpp2rust
- preprocessor/
- synthesizer/
  - src/
    - *.rs
    - CLAUDE.md (this)
- validator/
- data/
  - benchmark0/
    - cpp/
    - prepost/
    - processed/
    - stitched/
    - validated/
  - synth_benchmark/
    - cpp/
    - processed/
- README.md
- justfile
- .gitignore

### goals
the synthesizer should be able to quickly synthesize solutions for data/benchmark0/. "quickly" is not strictly defined, but 5 minutes for each test case would be a suitable upper limit. the intermediate goal is to quickly synthesize everything in data/synth_benchmark/ - these should be synthesized in 30s or less.
- [ ] synth_benchmark
