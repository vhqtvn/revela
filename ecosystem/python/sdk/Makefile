# Copyright (c) Aptos
# SPDX-License-Identifier: Apache-2.0

test:
	- poetry run python -m unittest discover -s aptos_sdk/ -p '*.py' -t ..

fmt:
	- find ./examples ./aptos_sdk -type f -name "*.py" | xargs poetry run autoflake -i -r --remove-all-unused-imports --remove-unused-variables --ignore-init-module-imports
	- find ./examples ./aptos_sdk -type f -name "*.py" | xargs poetry run isort
	- find ./examples ./aptos_sdk -type f -name "*.py" | xargs poetry run black

examples:
	- poetry run python -m examples.transfer-coin
	- poetry run python -m examples.simple-nft
	- poetry run python -m examples.transfer-two-by-two

.PHONY: test fmt examples
