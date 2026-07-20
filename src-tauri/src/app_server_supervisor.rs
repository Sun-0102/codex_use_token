use std::{
    io,
    time::{Duration, Instant},
};

use crate::app_server_session::AppServerSession;

pub trait ManagedSession {
    fn is_running(&mut self) -> io::Result<bool>;
    fn stop(&mut self) -> io::Result<()>;
}

impl ManagedSession for AppServerSession {
    fn is_running(&mut self) -> io::Result<bool> {
        AppServerSession::is_running(self)
    }

    fn stop(&mut self) -> io::Result<()> {
        AppServerSession::stop(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestartPolicy {
    initial_delay: Duration,
    max_delay: Duration,
}

impl RestartPolicy {
    pub fn new(initial_delay: Duration, max_delay: Duration) -> Self {
        Self {
            initial_delay,
            max_delay,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SupervisorEvent {
    Started,
    Running,
    Exited { next_restart_at: Instant },
    WaitingToRestart { next_restart_at: Instant },
    Restarted,
    Stopped,
    Shutdown,
}

pub struct AppServerSupervisor<F, S>
where
    S: ManagedSession,
{
    restart_policy: RestartPolicy,
    start_session: F,
    session: Option<S>,
    next_restart_at: Option<Instant>,
    next_restart_delay: Duration,
    shutdown: bool,
}

impl<F, S> AppServerSupervisor<F, S>
where
    F: FnMut() -> io::Result<S>,
    S: ManagedSession,
{
    pub fn new(restart_policy: RestartPolicy, start_session: F) -> Self {
        Self {
            restart_policy,
            start_session,
            session: None,
            next_restart_at: None,
            next_restart_delay: restart_policy.initial_delay,
            shutdown: false,
        }
    }

    pub fn start(&mut self) -> io::Result<SupervisorEvent> {
        if self.shutdown {
            return Ok(SupervisorEvent::Shutdown);
        }

        if self.session.is_some() {
            return Ok(SupervisorEvent::Running);
        }

        self.session = Some((self.start_session)()?);
        self.next_restart_at = None;
        Ok(SupervisorEvent::Started)
    }

    pub fn poll(&mut self, now: Instant) -> io::Result<SupervisorEvent> {
        if self.shutdown {
            return Ok(SupervisorEvent::Shutdown);
        }

        if let Some(next_restart_at) = self.next_restart_at {
            if now < next_restart_at {
                return Ok(SupervisorEvent::WaitingToRestart { next_restart_at });
            }

            self.session = Some((self.start_session)()?);
            self.next_restart_at = None;
            return Ok(SupervisorEvent::Restarted);
        }

        let Some(session) = self.session.as_mut() else {
            return self.start();
        };

        if session.is_running()? {
            return Ok(SupervisorEvent::Running);
        }

        self.session = None;
        let next_restart_at = now + self.next_restart_delay;
        self.next_restart_at = Some(next_restart_at);
        self.next_restart_delay =
            next_delay(self.next_restart_delay, self.restart_policy.max_delay);

        Ok(SupervisorEvent::Exited { next_restart_at })
    }

    pub fn shutdown(&mut self) -> io::Result<SupervisorEvent> {
        self.shutdown = true;
        self.next_restart_at = None;

        if let Some(mut session) = self.session.take() {
            session.stop()?;
            return Ok(SupervisorEvent::Stopped);
        }

        Ok(SupervisorEvent::Shutdown)
    }
}

impl<F, S> Drop for AppServerSupervisor<F, S>
where
    S: ManagedSession,
{
    fn drop(&mut self) {
        self.next_restart_at = None;

        if let Some(session) = self.session.as_mut() {
            let _ = session.stop();
        }
    }
}

fn next_delay(current: Duration, max_delay: Duration) -> Duration {
    current.saturating_mul(2).min(max_delay)
}
