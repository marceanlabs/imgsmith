# imgsmith benchmarks

Ubuntu 26.04, four cores, one 4032x3024 JPEG, median of three runs, against
ImageMagick 7.1.2. `scripts/bench.sh` reproduces the table.

| Task | imgsmith | ImageMagick |
|---|---|---|
| Resize to 1920 wide | 1.00 s | 1.13 s |
| Thumbnail 400 | 0.18 s | 0.29 s |
| JPEG to PNG, 12 megapixels | 6.65 s | 11.80 s |
| Installed size | one 6.6 MB file | 25 packages, 67 MB |

## Where it is not the faster tool

The table is not all one way, and it is worth saying so.

The resize win is 11%. Memory use is slightly worse than ImageMagick's. A JPEG
written at the same quality setting comes out larger, because imgsmith keeps the
full colour detail instead of discarding part of it.

What changes is the install: one file against 25 packages, and one piece of code
reading untrusted files instead of a stack of them.

## Reproducing it

```console
$ scripts/bench.sh
```
