The Large Grepowski
===================

Iterate over fragments of your code base and let a LLM evaluate your question on each fragment. Examine the results with
a nice tui.

Usage
-----

```
Usage: grepowski [OPTIONS] <QUESTION> <FILES>...

Arguments:
  <QUESTION>  Question to ask the model
  <FILES>...  Input files to analyze

Options:
  -l, --lines-per-block <LINES>       Number of lines per block [env: GREPOWSKI_LINES_PER_BLOCK=] [default: 20]
  -b, --blocks-per-fragment <BLOCKS>  Number of blocks per fragment [env: GREPOWSKI_BLOCKS_PER_FRAGMENT=] [default: 2]
  -m, --model <MODEL>                 Model to use for the chat completion [env: GREPOWSKI_MODEL=] [default: Qwen/Qwen2.5-Coder-3B-Instruct]
  -t, --temperature <TEMPERATURE>     Temperature for the chat completion [env: GREPOWSKI_TEMPERATURE=] [default: 0.2]
  -u, --url <URL>                     URL of the chat completion endpoint [env: GREPOWSKI_URL=] [default: http://127.0.0.1:9081/v1/chat/completions]
  -h, --help                          Print help
```

Build
-----

```
cargo build --release
```