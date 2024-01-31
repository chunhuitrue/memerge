#![allow(unused)]

use core::{future::Future, pin::Pin, task::{Context, Poll, RawWaker, RawWakerVTable, Waker}};

pub struct Task {
    future: Pin<Box<dyn Future<Output = ()>>>,
    state: TaskState
}

impl Task {
    pub fn new(future: impl Future<Output = ()> + 'static) -> Task {
        Task { future: Box::pin(future), state: TaskState::Start }
    }

    pub fn run(&mut self) {
        if self.state == TaskState::End {
            return;
        }
        
        let waker = dummy_waker();
        let mut context = Context::from_waker(&waker);
        match self.poll(&mut context) {
            Poll::Ready(()) => { self.state = TaskState::End }
            Poll::Pending => {}
        }
    }
    
    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context)
    }

    pub fn get_state(&self) -> TaskState {
        self.state
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TaskState {
    Start,
    End
}

fn dummy_raw_waker() -> RawWaker {
    fn no_op(_: *const ()) {}
    fn clone(_: *const ()) -> RawWaker {
        dummy_raw_waker()
    }

    let vtable = &RawWakerVTable::new(clone, no_op, no_op, no_op);
    RawWaker::new(std::ptr::null::<()>(), vtable)
}

fn dummy_waker() -> Waker {
    unsafe { Waker::from_raw(dummy_raw_waker()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task() {
        let mut task = Task::new(async_task());
        assert_eq!(TaskState::Start, task.get_state());        
        task.run();
        assert_eq!(TaskState::End, task.get_state());
        task.run();
        assert_eq!(TaskState::End, task.get_state());        
    }

    async fn async_task() {
        let number1 = async_number1().await;
        let number2 = async_number2().await;
        assert_eq!(1, number1);
        assert_eq!(2, number2);
    }
    
    async fn async_number1() -> u32 {
        1
    }

    async fn async_number2() -> u32 {
        2
    }
    
}
