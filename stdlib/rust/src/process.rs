use std::mem::{forget, transmute};

use serde::de::Deserialize;
use serde::ser::Serialize;

use crate::drop;
use crate::Channel;

mod stdlib {
    #[link(wasm_import_module = "lunatic")]
    extern "C" {
        pub fn spawn(
            function: unsafe extern "C" fn(usize, u64),
            argument1: usize,
            argument2: u64,
        ) -> u32;

        pub fn join(pid: u32);
    }
}

#[derive(Debug)]
pub struct SpawnError {}

/// A process consists of its own stack and heap. It can only share data with other processes by
/// exchanging the data with messages passing.
pub struct Process {
    id: u32,
}

impl Drop for Process {
    fn drop(&mut self) {
        drop(self.id);
    }
}

impl Process {
    /// Spawn a new process from a function and cotext.
    /// `function` is going to be starting point of the new process.
    /// `context` is some data that we want to pass to the newly spawned process.
    pub fn spawn<'de, T>(context: T, function: fn(T)) -> Result<Process, SpawnError>
    where
        T: Serialize + Deserialize<'de>,
    {
        unsafe extern "C" fn spawn_with_context<'de, T>(function: usize, channel: u64)
        where
            T: Serialize + Deserialize<'de>,
        {
            let channel: Channel<T> = Channel::dserialize_from_u64(channel);
            let context: T = channel.receive();
            let function: fn(T) = transmute(function);
            function(context);
        }

        let channel = Channel::new(1);
        channel.send(context);
        let serialized_channel = channel.serialize_as_u64();

        let id = unsafe {
            stdlib::spawn(
                spawn_with_context::<T>,
                transmute(function),
                serialized_channel,
            )
        };

        Ok(Self { id })
    }

    /// Wait on a process to finish.
    pub fn join(self) {
        unsafe {
            stdlib::join(self.id);
        };
        forget(self);
        // TODO: Drop externref
    }
}
