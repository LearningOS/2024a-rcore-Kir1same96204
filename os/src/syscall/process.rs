//! Process management syscalls
use crate::{
    config::MAX_SYSCALL_NUM,
    mm::{copy_to_translated_addr, mmap, munmap},
    task::{
        change_program_brk, current_user_token, exit_current_and_run_next, get_current_task_run_time, get_current_task_syscall_times, register_new_frame, suspend_current_and_run_next, unregister_frame, TaskStatus
    },
    timer::get_time,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time();
    let ts = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    copy_to_translated_addr(current_user_token(), &ts, _ts, 16);
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info");
    let ti = TaskInfo {
        status: TaskStatus::Running,
        syscall_times: get_current_task_syscall_times(),
        time: get_current_task_run_time(),
    };
    copy_to_translated_addr(current_user_token(), &ti, _ti as *mut TaskInfo, 2016);
    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap");

    match mmap(current_user_token(), _start, _len, _port) {
        Err(_) => -1,
        Ok(frames) => {
            for (vpn, frame) in frames.into_iter() {
                register_new_frame(vpn, frame);
            }

            0
        },
    }
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    
    match munmap(current_user_token(), _start, _len) {
        Err(_) => -1,
        Ok(unmmaped_vpns) => {
            for vpn in unmmaped_vpns.into_iter() {
                unregister_frame(vpn);
            }

            0
        }
    }
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
