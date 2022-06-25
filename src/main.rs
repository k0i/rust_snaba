use std::{
    collections::{HashMap, VecDeque},
    os::unix::prelude::RawFd,
    sync::{
        mpsc::{sync_channel, Receiver, SyncSender},
        Arc, Mutex,
    },
    task::{Context, Waker},
};

use futures::{
    future::BoxFuture,
    task::{waker_ref, ArcWake},
    Future, FutureExt,
};
use nix::{
    errno::Errno,
    sys::{
        epoll::{
            epoll_create1, epoll_ctl, epoll_wait, EpollCreateFlags, EpollEvent, EpollFlags, EpollOp,
        },
        eventfd::{eventfd, EfdFlags},
    },
    unistd::write,
    Error,
};

fn main() {
    let executor = Executor::new();
    executor.get_spawner().spawn(Hello::new());
    executor.run();
}

struct Task {
    future: Mutex<BoxFuture<'static, ()>>,
    sender: SyncSender<Arc<Task>>,
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self.sender.send(arc_self.clone()).unwrap();
    }
}

struct Executor {
    sender: SyncSender<Arc<Task>>,
    receiver: Receiver<Arc<Task>>,
}

struct Spawner {
    sender: SyncSender<Arc<Task>>,
}

impl Executor {
    fn new() -> Self {
        let (sender, receiver) = sync_channel(1024);
        Self { sender, receiver }
    }
    fn get_spawner(&self) -> Spawner {
        Spawner {
            sender: self.sender.clone(),
        }
    }
    fn run(&self) {
        while let Ok(task) = self.receiver.recv() {
            let mut future = task.future.lock().unwrap();
            let waker = waker_ref(&task);
            let mut context = Context::from_waker(&waker);
            let _ = future.as_mut().poll(&mut context);
        }
    }
}

impl Spawner {
    fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        let future = future.boxed();
        let task = Arc::new(Task {
            future: Mutex::new(future),
            sender: self.sender.clone(),
        });
        self.sender.send(task).unwrap();
    }
}
enum StateHello {
    Hello,
    World,
    Fin,
}

struct Hello {
    state: StateHello,
}
impl Hello {
    fn new() -> Self {
        Self {
            state: StateHello::Hello,
        }
    }
}

impl Future for Hello {
    type Output = ();
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match &mut self.state {
            StateHello::Hello => {
                println!("Hello");
                self.state = StateHello::World;
                cx.waker().wake_by_ref();
                std::task::Poll::Pending
            }
            StateHello::World => {
                println!("World");
                self.state = StateHello::Fin;
                cx.waker().wake_by_ref();
                std::task::Poll::Pending
            }
            StateHello::Fin => {
                println!("FIN");
                std::task::Poll::Ready(())
            }
        }
    }
}

fn write_eventfd(fd: RawFd, n: usize) {
    let ptr = &n as *const usize as *const u8;
    let val = unsafe { std::slice::from_raw_parts(ptr, std::mem::size_of_val(&n)) };
    write(fd, val).unwrap();
}

enum IOOps {
    ADD(EpollFlags, RawFd, Waker),
    REMOVE(RawFd),
}
struct IOSelector {
    wakers: Mutex<HashMap<RawFd, Waker>>,
    queue: Mutex<VecDeque<IOOps>>,
    epfd: RawFd,
    event: RawFd,
}
impl IOSelector {
    fn new() -> Arc<Self> {
        let s = Self {
            wakers: Mutex::new(HashMap::new()),
            queue: Mutex::new(VecDeque::new()),
            epfd: epoll_create1(EpollCreateFlags::empty()).unwrap(),
            event: eventfd(0, EfdFlags::empty()).unwrap(),
        };
        let result = Arc::new(s);
        let s = result.clone();
        std::thread::spawn(move || s.select());
        result
    }
    fn add_event(
        &self,
        flag: EpollFlags,
        fd: RawFd,
        waker: Waker,
        wakers: &mut HashMap<RawFd, Waker>,
    ) {
        let epoll_add = EpollOp::EpollCtlAdd;
        let epoll_mod = EpollOp::EpollCtlMod;
        let epoll_one = EpollFlags::EPOLLONESHOT;
        let mut ev = EpollEvent::new(flag | epoll_one, fd as u64);
        if let Err(e) = epoll_ctl(self.epfd, epoll_add, fd, &mut ev) {
            match e {
                Errno::EEXIST => {
                    epoll_ctl(self.epfd, epoll_mod, fd, &mut ev).unwrap();
                }
                _ => panic!("epoll_ctl:{}", e),
            }
        }
        assert!(wakers.contains_key(&fd));
        wakers.insert(fd, waker);
    }
    fn rm_event(&self, fd: RawFd, wakers: &mut HashMap<RawFd, Waker>) {
        let epoll_del = EpollOp::EpollCtlDel;
        let mut ev = EpollEvent::new(EpollFlags::empty(), fd as u64);
        epoll_ctl(self.epfd, epoll_del, fd, &mut ev).unwrap();
        wakers.remove(&fd);
    }
    fn select(&self) {
        let epoll_in = EpollFlags::EPOLLIN;
        let epoll_add = EpollOp::EpollCtlAdd;
        let mut ev = EpollEvent::new(epoll_in, self.event as u64);
        epoll_ctl(self.epfd, epoll_add, self.event, &mut ev).unwrap();
        let mut events = vec![EpollEvent::empty(); 1024];
        while let Ok(nfds) = epoll_wait(self.epfd, &mut events, -1) {
            for n in 0..nfds {
                if events[n].data() == self.event as u64 {
                    let mut q = self.queue.lock().unwrap();
                    while let Some(op) = q.pop_front() {
                        match op {
                            IOOps::ADD(flag, fd, waker) => {
                                self.add_event(flag, fd, waker, &mut self.wakers.lock().unwrap());
                            }
                            IOOps::REMOVE(fd) => {
                                self.rm_event(fd, &mut self.wakers.lock().unwrap());
                            }
                        }
                    }
                }
            }
        }
    }
}
