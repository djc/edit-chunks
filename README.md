# edit-chunks

Splits byte ranges out of a (large) file and writes them into separate files
for convenient editing. Then combines the original and edited parts again.

Help output:

```
edit-chunks 0.1.0
Dirkjan Ochtman <first (at) last . nl>
Split out chunks of a large file for editing,
then put them back together again.

USAGE:
    edit-chunks <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    combine    combine a previously split file again
    help       Prints this message or the help of the given subcommand(s)
    split      split a file
```

Example command-line:

```shell
$ ./edit-chunks split large-file.dat 1429423-1439934
$ $EDITOR large-file.dat.part.0
$ ./edit-chunks combine large-file.dat.spec
```

All feedback welcome in the GitHub [issue tracker](https://github.com/djc/edit-chunks/issues).
