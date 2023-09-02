// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::thread_manager::ThreadManager;
use aptos_runtimes::spawn_rayon_thread_pool;
use rayon::ThreadPool;

pub struct DefaultThreadManager {
    exe_threads: ThreadPool,
    non_exe_threads: ThreadPool,
    io_threads: ThreadPool,
}

impl DefaultThreadManager {
    pub(crate) fn new() -> DefaultThreadManager {
        let exe_threads = spawn_rayon_thread_pool("exe".into(), Some(num_cpus::get()));
        let non_exe_threads = spawn_rayon_thread_pool("non_exe".into(), Some(num_cpus::get()));
        let io_threads = spawn_rayon_thread_pool("io".into(), Some(64));
        Self {
            exe_threads,
            non_exe_threads,
            io_threads,
        }
    }
}

impl<'a> ThreadManager<'a> for DefaultThreadManager {
    fn get_exe_cpu_pool(&'a self) -> &'a ThreadPool {
        &self.exe_threads
    }

    fn get_non_exe_cpu_pool(&'a self) -> &'a ThreadPool {
        &self.non_exe_threads
    }

    fn get_io_pool(&'a self) -> &'a ThreadPool {
        &self.io_threads
    }

    fn get_high_pri_io_pool(&'a self) -> &'a ThreadPool {
        &self.io_threads
    }
}
