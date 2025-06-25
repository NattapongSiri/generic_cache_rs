# generic_cache
A generic cached object which provide user two possible usage options.
1. Use `Object::get()` until it return `TimeoutError` then manually call `Object::refresh()` function.
1. Use `Object::get_or_refresh()` which will automatically refresh the value when it is expired.

There is a special case that require explicit type declaration.
In such case, since version 1.1.0, it add trait `CachedObject` which required only two generic type which is a type being cache and the error type which may occur during value refresh. The type of refresh function is no longer required. 

You do **not** need to use this trait to use cache. Only `Object` struct is all you need. The trait is just to workaround this specific case.

By current Rust limitation, it is currently impossible to use `impl trait` on let/static binding.
See this [RFC](https://github.com/rust-lang/rust/issues/63065) for detail.

With this trait, it allow usage in static context by utilizing `dyn trait`.
An example of such usage is:
```rust
use core::time::Duration;
use generic_cache::{CachedObject, Object};
use std::sync::{LazyLock, RwLock};
use tokio::time::sleep;

static CACHED: LazyLock<RwLock<Box<dyn CachedObject<u16, ()> + Send + Sync>>> = LazyLock::new(|| {
   RwLock::new(Box::new(Object::new(std::time::Duration::from_secs(1), 100, async || {Ok::<u16, ()>(200)})))
});
assert_eq!((&*CACHED).read().unwrap().get().unwrap(), &100u16);
sleep(Duration::from_secs(2)).await;
assert!((&*CACHED).read().unwrap().get().is_err(), "Cache should be expired");
assert_eq!((&*CACHED).write().unwrap().get_or_refresh().await.unwrap(), &200u16, "Cache should be refreshed to 200");
```

It is important to note that the trait provides mirror function to the original function provided by `Object` with two differents. Both method `refresh` and `get_or_refresh` return `Pin<Box<dyn Future>>` instead of `impl Future`.
This mean that the trait return heap allocated pinned future whereas `Object` return `impl Future` which may or may not be on heap. This is a trade-off that need to be made to make it dyn compatible.

If this [RFC](https://github.com/rust-lang/rfcs/pull/3546) is resolved, it will allow omitting the type declaration altogether if there's no ambiguity type inference occur.

## Rationale
For performance critical application, most of the time, major performance cost came from I/O. To reduce cost, the easiest way is to cache the value. In some case, it is possible to delegate this work to network layer, e.g. Proxy. In some other case, it is not possible due to security reason. An example of such case is the bearer token which is used to communicate between API server. It is normally obtained via HTTP POST which proxy won't cache. In such case, some vendor provide a library which handle token caching but it is not always the case. This is where this library fit in.

## Breaking change
### Version 0.3.0
- Change `ttl` argument type from `u128` to `std::time::Duration` type.