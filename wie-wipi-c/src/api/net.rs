use wipi_types::wipic::WIPICWord;

use crate::{WIPICResult, method::MethodBody};
use alloc::{boxed::Box, vec::Vec};
use wie_util::{Result, WieError};

use crate::context::WIPICContext;

/// Accept valid asynchronous requests and complete them once with offline failure.
pub async fn connect(context: &mut dyn WIPICContext, cb: WIPICWord, param: WIPICWord) -> Result<i32> {
    if cb == 0 {
        return Ok(-1);
    }
    struct Completion {
        cb: u32,
        param: u32,
    }
    #[async_trait::async_trait]
    impl MethodBody<WieError> for Completion {
        async fn call(&self, context: &mut dyn WIPICContext, _: Box<[WIPICWord]>) -> Result<WIPICResult> {
            context.call_function(self.cb, &[u32::MAX, self.param]).await?;
            Ok(WIPICResult { results: Vec::new() })
        }
    }
    context.spawn(Box::new(Completion { cb, param }))?;
    Ok(0)
}

pub async fn close(_context: &mut dyn WIPICContext) -> Result<()> {
    tracing::warn!("stub MC_netClose()");

    Ok(())
}

pub async fn socket_close(_context: &mut dyn WIPICContext, fd: i32) -> Result<i32> {
    tracing::warn!("stub MC_netSocketClose({fd})");

    Ok(-1) // M_E_ERROR
}

/// The offline backend never establishes sockets; writes fail synchronously.
/// Do not report bytes sent or dereference a buffer for a disconnected socket.
pub async fn socket_write(_context: &mut dyn WIPICContext, fd: i32, _buffer: WIPICWord, length: i32) -> Result<i32> {
    tracing::debug!("MC_netSocketWrite({fd}, length={length}) -> M_E_ERROR (offline)");
    Ok(-1)
}

/// No socket is created by the offline backend. Let the guest handle failure.
pub async fn socket(_context: &mut dyn WIPICContext, _domain: i32, _kind: i32) -> Result<i32> {
    Ok(-1)
}

/// Read/write readiness registration cannot succeed without an online socket.
/// In particular, NULL unregister requests must not call guest address zero.
pub async fn set_socket_callback(_context: &mut dyn WIPICContext, _fd: i32, _callback: u32, _parameter: u32) -> Result<i32> {
    Ok(-14) // M_E_NOTCONN
}

pub async fn socket_read(_context: &mut dyn WIPICContext, _fd: i32, _buffer: u32, _length: i32) -> Result<i32> {
    Ok(-14) // M_E_NOTCONN; leave the destination untouched.
}

pub async fn socket_connect(_context: &mut dyn WIPICContext, _fd: i32, _address: u32, _port: u32, callback: u32, _parameter: u32) -> Result<i32> {
    Ok(if callback == 0 { -9 } else { -14 }) // M_E_INVALID / M_E_NOTCONN
}

#[cfg(test)]
mod tests {
    use crate::context::test::TestContext;
    #[futures_test::test]
    async fn offline_socket_operations_never_schedule_or_access_guest_buffers() {
        let mut context = TestContext::new();
        for callback in [0, 0x101] {
            assert_eq!(super::set_socket_callback(&mut context, 0, callback, 42).await.unwrap(), -14);
            assert_eq!(
                super::socket_connect(&mut context, 0, 0, 80, callback, 42).await.unwrap(),
                if callback == 0 { -9 } else { -14 }
            );
        }
        assert_eq!(super::socket_read(&mut context, 0, u32::MAX, i32::MAX).await.unwrap(), -14);
        assert!(context.spawned.is_empty());
        assert!(context.calls.is_empty());
        assert_eq!(context.io_counts(), (0, 0));
    }
    #[futures_test::test]
    async fn offline_connection_completes_once_after_acceptance() {
        let mut context = TestContext::new();
        assert_eq!(super::connect(&mut context, 0, 123).await.unwrap(), -1);
        assert!(context.spawned.is_empty());
        assert_eq!(super::connect(&mut context, 0x101, 123).await.unwrap(), 0);
        assert!(context.calls.is_empty());
        assert_eq!(context.spawned.len(), 1);
        let callback = context.spawned.pop().unwrap();
        callback.call(&mut context, alloc::vec![].into_boxed_slice()).await.unwrap();
        assert_eq!(context.calls, alloc::vec![(0x101, alloc::vec![u32::MAX, 123])]);
        assert!(context.spawned.is_empty());
    }
}
