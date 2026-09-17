# Typed ticket operations

The live implementation is `../ops/`. It validates post-images before applying changes and returns the exact changed/deleted ticket IDs for surgical persistence through `Corpus`. CLI orchestration lives in `../cli/`.
