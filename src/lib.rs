//! A generic cached object which provide user two possible usage options.
//! 1. Use [Object::get()] until it return [TimeoutError] then manually call [Object::refresh()] function.
//! 1. Use [Object::get_or_refresh()] which will automatically refresh the value when it is expired.
//! 
//! The different between the two is that the [Object::get()] is more flexible because it only borrow
//! the cache value while the [Object::get_or_refresh()] will required a borrow mut of [Object] itself because it
//! might need to change the cached value. However, the auto refresh is convenient because user doesn't
//! need to handle [TimeoutError] when cache is expired.
//! Both usage options still need to handle `refresh_fn` error if any.
//! 
//! # Example
//! - Verify two cached call to get value back to back to check if it is actually the same value.
//! ```rust
//! use generic_cache::Object;
//! 
//! let cached = Object::new(1000, 100, async || {Ok(200)});
//! let first = cached.get().unwrap();
//! let second = cached.get().unwrap();
//! assert_eq!(*first, 100, "Expect {} to equals {}", *first, 0);
//! assert_eq!(first, second, "Expect {} to equals {}", first, second);
//! ```
//! - Check for expired then refresh the cache
//! ```rust
//! use core::time;
//! use std::thread::sleep;
//! use generic_cache::Object;
//! 
//! # tokio_test::block_on(async {
//! let mut cached = Object::new(0, 100, async || {Ok(200)});
//! let first = *cached.get().unwrap();
//! sleep(time::Duration::from_millis(1));
//! if let Ok(_) = cached.get() {
//!     panic!("Cache should be expired but it is not.")
//! } else {
//!     cached.refresh().await.unwrap();
//! }
//! let second = *cached.get().unwrap();
//! assert_ne!(first, second, "Expect {} to equals {}", first, second);
//! # })
//! ```
//! - Auto refresh expired cache value
//! ```rust
//! use core::time;
//! use std::thread::sleep;
//! use generic_cache::Object;
//! 
//! # tokio_test::block_on(async {
//! let mut cached = Object::new(0, 100, async || {Ok(200)});
//! let first = *cached.get_or_refresh().await.unwrap();
//! sleep(time::Duration::from_millis(1));
//! let second = *cached.get_or_refresh().await.unwrap();
//! assert_ne!(first, second, "Expect {} to equals {}", first, second);
//! # })
//! ```
//! - No default value when create a cache and auto refresh expired cache value
//! ```rust
//! use core::time;
//! use std::thread::sleep;
//! use generic_cache::Object;
//! 
//! # tokio_test::block_on(async {
//! let mut cached = Object::new_and_refresh(1000, async || {Ok(200)}).await.unwrap();
//! let first = *cached.get_or_refresh().await.unwrap();
//! let second = *cached.get_or_refresh().await.unwrap();
//! assert_eq!(first, second, "Expect {} to equals {}", first, second);
//! # })
//! ```

use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::ops::AsyncFn;
use std::time::SystemTime;
/** The cache is timeout. [Object::refresh()] need to be called. */
pub struct TimeoutError {}
impl Display for TimeoutError {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "The cached object is timeout. Please call refresh method to refresh the value.")
    }
}
impl Debug for TimeoutError {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "The cached object is timeout. Please call refresh method to refresh the value.")
    }
}

/**
 * Generic cache object which cache an object for given period of time before it return TimeoutError
 * to signal caller to call refresh function before further attempt.
 * The refresh_fn should be async function that return Result of the same type as the cached object.
 * If there's any error occur inside refresh_fn, it should return Error result back.
 */
pub struct Object<T, F> where F: AsyncFn() -> Result<T, Box<dyn Error>> {
    ttl: u128,
    last_update: SystemTime,
    obj: T,
    refresh_fn: F
}
impl<T, F> Debug for Object<T, F> where T: Debug, F: AsyncFn() -> Result<T, Box<dyn Error>> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(fmt, "{{ttl: {}, elapsed: {}, obj: {:#?}}}", self.ttl, self.last_update.elapsed().unwrap().as_millis(), self.obj)
    }
}
impl<T, F> Object<T, F> where F: AsyncFn() -> Result<T, Box<dyn Error>> {
    /** 
     * Create a new cached Object with default value specify in second argument. 
     * `ttl` is in milli-second unit.
     * `refresh_fn` is a function to refresh value and last update time.
     */
    pub fn new(ttl: u128, obj: T, refresh_fn: F) -> Object<T, F> {
        Object {
            ttl,
            last_update: SystemTime::now(),
            obj,
            refresh_fn
        }
    }
    /**
     * Create a new cached Object and immediately refresh the value instead of using default value.
     * `ttl` is in milli-second unit.
     * `refresh_fn` is a function to refresh value and last update time.
     * The different from `new` function is that it is async and it immediately call `refresh_fn`.
     */
    pub async fn new_and_refresh(ttl: u128, refresh_fn: F) -> Result<Object<T, F>, Box<dyn Error>> {
        let v = refresh_fn().await?;
        let obj = Object {
            ttl,
            last_update: SystemTime::now(),
            obj: v,
            refresh_fn
        };
        Ok(obj)
    }
    /**
     * Refresh cache immediately and update last update time if refresh success.
     */
    pub async fn refresh(&mut self) -> Result<(), Box<dyn Error>> {
        self.obj = (self.refresh_fn)().await?;
        self.last_update = SystemTime::now();
        Ok(())
    }
    /**
     * Read current cached value or return Error if cache is already expired.
     */
    pub fn get(&self) -> Result<&T, TimeoutError> {
        if self.last_update.elapsed().unwrap().as_millis() > self.ttl {
            return Err(TimeoutError {})
        }
        Ok(&self.obj)
    }
    /**
     * Read current cached value or refresh the value if it is already expired then
     * return the new value.
     */
    pub async fn get_or_refresh(&mut self) -> Result<&T, Box<dyn Error>> {
        if self.last_update.elapsed().unwrap().as_millis() > self.ttl {
            self.obj = (self.refresh_fn)().await?;
        }
        Ok(&self.obj)
    }
}

#[cfg(test)]
mod tests {
    use core::time;
    use std::thread::sleep;

    use super::*;

    #[test]
    fn simple_cache() {
        let cached = Object::new(1000, 100, async || {Ok(200)});
        let first = cached.get().unwrap();
        let second = cached.get().unwrap();
        assert_eq!(*first, 100, "Expect {} to equals {}", *first, 0);
        assert_eq!(first, second, "Expect {} to equals {}", first, second);
    }
    #[tokio::test]
    async fn simple_refresh() {
        let mut cached = Object::new(1000, 100, async || {Ok(200)});
        let first = *cached.get().unwrap();
        cached.refresh().await.unwrap();
        let second = *cached.get().unwrap();
        assert_eq!(first, 100, "Expect {} to equals {}", first, 100);
        assert_eq!(second, 200, "Expect {} to equals {}", first, 200);
    }
    #[tokio::test]
    async fn simple_no_cache() {
        let mut cached = Object::new(0, 100, async || {Ok(200)});
        let first = *cached.get_or_refresh().await.unwrap();
        sleep(time::Duration::from_millis(1));
        let second = *cached.get_or_refresh().await.unwrap();
        assert_ne!(first, second, "Expect {} to equals {}", first, second);
    }
    #[tokio::test]
    async fn simple_expire_check() {
        let mut cached = Object::new(0, 100, async || {Ok(200)});
        let first = *cached.get().unwrap();
        sleep(time::Duration::from_millis(1));
        if let Ok(_) = cached.get() {
            panic!("Cache should be expired but it is not.")
        } else {
            cached.refresh().await.unwrap();
        }
        let second = *cached.get().unwrap();
        assert_ne!(first, second, "Expect {} to equals {}", first, second);
    }
    #[tokio::test]
    async fn immediate_refresh() {
        let mut cached = Object::new_and_refresh(1000, async || {Ok(200)}).await.unwrap();
        let first = *cached.get_or_refresh().await.unwrap();
        let second = *cached.get_or_refresh().await.unwrap();
        assert_eq!(first, second, "Expect {} to equals {}", first, second);
    }
    #[test]
    fn simple_object() {
        struct Dummy {
            v: u8
        }
        let cached = Object::new(1000, Dummy {v: 1}, async || {Ok(Dummy {v: 2})});
        let Dummy { v: v1} = cached.get().unwrap();
        let Dummy { v: v2} = cached.get().unwrap();
        assert_eq!(*v1, 1, "Expect {} to equals {}", v1, 1);
        assert_eq!(v1, v2, "Expect {} to equals {}", v1, v2);
    }
}