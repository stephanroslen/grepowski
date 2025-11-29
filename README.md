The Large Grepowski
===================

Iterate over fragments of your code base and let a LLM evaluate your question on each fragment. Examine the results with
a nice tui.

Installation
------------

```
cargo install --git https://github.com/stephanroslen/grepowski.git
```

Usage
-----

```
Usage: grepowski ask [OPTIONS] --model <MODEL> <QUESTION> <FILES>...

Arguments:
  <QUESTION>  Question to ask the model
  <FILES>...  Input files to analyze

Options:
  -l, --lines-per-block <LINES>       Number of lines per block [env: GREPOWSKI_LINES_PER_BLOCK=] [default: 10]
  -b, --blocks-per-fragment <BLOCKS>  Number of blocks per fragment [env: GREPOWSKI_BLOCKS_PER_FRAGMENT=] [default: 3]
  -m, --model <MODEL>                 Model to use for the chat completion [env: GREPOWSKI_MODEL=]
  -t, --temperature <TEMPERATURE>     Temperature for the chat completion [env: GREPOWSKI_TEMPERATURE=] [default: 0.0]
  -u, --url <URL>                     URL of the chat completion endpoint [env: GREPOWSKI_URL=] [default: http://127.0.0.1:8080/v1]
  -a, --auth-token <TOKEN>            Bearer token for the chat completion endpoint - if not set, the model will be used anonymously [env: GREPOWSKI_AUTH_TOKEN]
  -h, --help                          Print help
```

Completions
-----------

```
Usage: grepowski completions <SHELL>

Arguments:
  <SHELL>  Shell to generate completions for [possible values: bash, elvish, fish, powershell, zsh]

Options:
  -h, --help  Print help
```

E.g. for zsh `eval "$(grepowski completions zsh)"`.
