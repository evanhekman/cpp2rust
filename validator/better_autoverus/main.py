# Copyright (c) Microsoft Corporation. #
# Licensed under the MIT license.      #


import argparse
import json
import logging
import os

from veval import verus

import utils
from utils import AttrDict


def main():
    # Parse arguments.
    parser = argparse.ArgumentParser(description="Verus Copilot")
    parser.add_argument("--config", default="config.json", help="Path to config file")
    parser.add_argument("--input", default="input.rs", help="Path to input file")
    parser.add_argument("--output", default="output.rs", help="Path to output file")
    parser.add_argument("--temp", default=1.0, type=float, help="The temperature for LLM")
    parser.add_argument("--disable-safe", action="store_true", help="Disable safe check for code")
    parser.add_argument(
        "--examples",
        default=[3],
        nargs="+",
        help="Examples to be given to LLM",
    )

    args = parser.parse_args()
    # Set log level.
    logging.getLogger("httpx").setLevel(logging.WARNING)
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s %(levelname)s: %(message)s",
        datefmt="%Y-%m-%d %H:%M:%S",
    )
    logger = logging.getLogger(__name__)

    # Check if config file exists
    if not os.path.isfile(args.config):
        logger.error("Config file does not exist")
        return

    # Check if input file exists
    if not os.path.isfile(args.input):
        logger.error("Input file does not exist")
        return

    config = json.load(open(args.config))
    config = AttrDict(config)
    verus.set_verus_path(config.verus_path)

    # Config
    if args.disable_safe:
        logger.warning("Safe check for code is disabled!!!")
        utils.DEBUG_SAFE_CODE_CHANGE = True

    logger.info("Examples currently being used: %s", args.examples)

    # Run the appropriate mode.
    logger.info("Running in generation mode")
    from generation import Generation

    runner = Generation(
        config,
        logger,
        examples=args.examples,
    )

    runner.run(args.input, args.output, args=dict(args._get_kwargs()))


if __name__ == "__main__":
    main()
