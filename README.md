# generic_cache
A generic cached object which provide user two possible usage options.
1. Use `Object::get()` until it return `TimeoutError` then manually call `Object::refresh()` function.
1. Use `Object::get_or_refresh()` which will automatically refresh the value when it is expired.

## Rationale
For performance critical application, most of the time, major cost came from IO. To reduce cost, the easiest way is to cache the value. In some case, it is possible to delegate this work to network layer, e.g. Proxy. In some other case, it is not possible due to security reason. An example of such case is the bearer token which is used to communicate between API server. It is normally obtained via HTTP POST which proxy won't cache. In such case, some vendor provide a library which handle token caching but it is not always the case. This is where this library fit in.