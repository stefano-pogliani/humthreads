# Threads for Humans (DEPRECATED)

**NOTE**: this library is now deprecated and won't be updated much (not like it was until now ...).

A rust library built on top of [`std::thread`](https://doc.rust-lang.org/stable/std/thread/)
to provide additional features and tools to make development easier and to support operators.

Threads are a complex beast.
Rust does a great job at providing an efficient, cross-platform, support for essential operations.
But larger and more complex projects need a bit more.

The `humthreads` library provides the following additional features:

* Signal threads they should shutdown.
* Join threads with timeouts instead of blocking forever if a thread is not done.
* Introspection API to aid debugging and monitoring multi-threaded processes.

## Code of Conduct

Our aim is to build a thriving, healthy and diverse community.  
To help us get there we decided to adopt the [Contributor Covenant Code of Conduct](https://www.contributor-covenant.org/)
for all our projects.

Any issue should be reported to [stefano-pogliani](https://github.com/stefano-pogliani)
by emailing [conduct@replicante.io](mailto:conduct@replicante.io).  
Unfortunately, as the community lucks members, we are unable to provide a second contact to report incidents to.  
We would still encourage people to report issues, even anonymously.
