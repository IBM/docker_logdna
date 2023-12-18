use async_channel::{bounded, unbounded};
use async_channel::{Receiver, Sender};
use thiserror::Error;

use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use bytes::buf::Buf;

#[derive(Error, std::fmt::Debug)]
pub enum PoolError {
    #[error("Failed to attach object to pool")]
    AttachError,
    #[error("No buffers available in pool")]
    NoBuffersAvailable,
    #[error("Initial Capacity larger than Max Reserve Capacity")]
    InitError(usize, usize),
}

pub trait ClearBuf {
    fn clear(&mut self);
}

pub struct Pool<F, T> {
    object_bucket: Receiver<T>,
    object_return: Sender<T>,
    extend_fn: F,
}

impl<F, T> Clone for Pool<F, T>
where
    F: Clone,
{
    fn clone(&self) -> Self {
        Pool {
            object_bucket: self.object_bucket.clone(),
            object_return: self.object_return.clone(),
            extend_fn: self.extend_fn.clone(),
        }
    }
}

impl<F, T: ClearBuf + std::marker::Send> Pool<Arc<F>, T>
where
    F: Fn() -> T + std::marker::Send + std::marker::Sync + 'static + ?Sized,
{
    #[inline]
    pub fn new(initial_capacity: usize, init: Arc<F>) -> Self {
        let (s, r) = unbounded();

        for _ in 0..initial_capacity {
            s.try_send(init()).expect("Pool is closed");
        }

        Pool {
            object_bucket: r,
            object_return: s,
            extend_fn: init,
        }
    }

    /// Create a Pool with a limited reserve capacity unused objects
    ///
    /// If the Pool of available objects reaches the maximum reserve capacity
    /// any additional Reusable handles that are dropped will drop their
    /// internal object rather than returning it to the pool
    #[inline]
    pub fn with_max_reserve(
        initial_capacity: usize,
        max_reserve_capacity: usize,
        init: Arc<F>,
    ) -> Result<Self, PoolError> {
        if max_reserve_capacity < initial_capacity {
            return Err(PoolError::InitError(initial_capacity, max_reserve_capacity));
        }
        let (s, r) = bounded(max_reserve_capacity);

        for _ in 0..initial_capacity {
            s.try_send(init()).expect("Pool is closed");
        }

        Ok(Pool {
            object_bucket: r,
            object_return: s,
            extend_fn: init,
        })
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.object_bucket.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.object_bucket.is_empty()
    }

    #[inline]
    pub async fn pull(&self) -> Option<Reusable<T>> {
        self.object_bucket
            .recv()
            .await
            .ok()
            .map(|data| Reusable::new(self.object_return.clone(), data))
    }

    #[inline]
    pub fn try_pull(&self) -> Result<Reusable<T>, PoolError> {
        self.object_bucket
            .try_recv()
            .map(|data| Reusable::new(self.object_return.clone(), data))
            .map_err(|_| /*TODO handle the real errors*/ PoolError::NoBuffersAvailable)
    }

    #[inline]
    pub async fn attach(&self, t: T) -> Result<(), PoolError> {
        self.object_return
            .send(t)
            .await
            .map_err(|_| PoolError::AttachError)
    }

    #[inline]
    pub fn try_attach(&self, t: T) -> Result<(), PoolError> {
        self.object_return
            .try_send(t)
            .map_err(|_| PoolError::AttachError)
    }

    #[inline]
    pub fn expand(&mut self) -> Result<(), PoolError> {
        self.try_attach((self.extend_fn)())
    }
}

pub struct Reusable<T: ClearBuf> {
    pool: Sender<T>,
    data: ManuallyDrop<T>,
    // Is there something in data to drop?
    present: bool,
}

impl<'a, T: ClearBuf> Reusable<T> {
    #[inline]
    pub fn new(pool: Sender<T>, t: T) -> Self {
        Self {
            pool,
            data: ManuallyDrop::new(t),
            present: true,
        }
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.data.deref_mut()
    }

    #[inline]
    pub fn detach(mut self) -> (Sender<T>, T) {
        (self.pool.clone(), unsafe { self.take() })
    }

    #[inline]
    pub fn into_inner(mut self) -> T {
        unsafe { self.take() }
    }

    unsafe fn take(&mut self) -> T {
        self.present = false;
        ManuallyDrop::take(&mut self.data)
    }
}

impl<'a, T: ClearBuf> Deref for Reusable<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<'a, T: ClearBuf> DerefMut for Reusable<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<'a, T: ClearBuf> Drop for Reusable<T> {
    #[inline]
    fn drop(&mut self) {
        if self.present {
            let mut obj = unsafe { self.take() };
            // If we can't put it back on the pool drop it
            obj.clear();
            match self.pool.try_send(obj) {
                Ok(_) => {}
                Err(e) => drop(e),
            };
        }
    }
}

impl<T> std::convert::AsRef<[u8]> for Reusable<T>
where
    T: std::convert::AsRef<[u8]> + ClearBuf,
{
    fn as_ref(&self) -> &[u8] {
        self.data.as_ref()
    }
}

impl<T: ClearBuf> Buf for Reusable<T>
where
    T: Buf,
{
    fn remaining(&self) -> usize {
        self.data.remaining()
    }

    fn chunk(&self) -> &[u8] {
        self.data.chunk()
    }

    fn advance(&mut self, cnt: usize) {
        self.data.advance(cnt)
    }
}
