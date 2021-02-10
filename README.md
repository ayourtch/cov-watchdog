# cov-watchdog
Watchdog for finding the owners for coverity-scan defects based on MAINTAINERS file

```
$ ./target/debug/cov-watchdog --help
cov-watchdog version tags/v0.0.1-0-g2215ea9
Andrew Yourtchenko <ayourtch@gmail.com>
read the json data from Coverity and do something with it

USAGE:
    cov-watchdog [FLAGS] [OPTIONS] --in-file <in-file> --maintainers-file <maintainers-file>

FLAGS:
    -h, --help       Prints help information
    -v, --verbose    A level of verbosity, and can be used multiple times
    -V, --version    Prints version information

OPTIONS:
    -c, --component-word <component-word>...     Show the bugs that match component(s) (exact match)
    -i, --in-file <in-file>
            Input JSON file name saved from a URL similar to
            https://scan9.coverity.com/api/viewContents/issues/v1/28863?projectId=12999&rowCount=-1

    -m, --maintainers-file <maintainers-file>    MAINTAINERS file name
    -p, --person <person>...
            Show the bugs that match maintainer(s) (substring search)

```

