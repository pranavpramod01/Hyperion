use crate::module::Result;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// Minimum job representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub id: u64,
    pub kind: String,
    pub payload: String,
    pub created_at: Instant,
    pub attempts: u32,
}

/// Leased job with expiration.
#[derive(Debug, Clone)]
struct Lease {
    job: Job,
    expires_at: Instant,
}

/// In-memory scheduler with lease and retry support.
pub struct Scheduler {
    next_id: u64,
    queued: HashMap<String, VecDeque<Job>>,
    leased: HashMap<u64, Lease>,
    done: Vec<Job>,
    failed: Vec<Job>,
    default_kind: String,
}

impl Scheduler {
    /// Create a new scheduler with a default job kind.
    pub fn new() -> Self {
        Self {
            next_id: 1,
            queued: HashMap::new(),
            leased: HashMap::new(),
            done: Vec::new(),
            failed: Vec::new(),
            default_kind: "default".into(),
        }
    }

    /// Enqueue a job to a given kind.
    pub fn enqueue<S: Into<String>, P: Into<String>>(&mut self, kind: S, payload: P) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let job = Job {
            id,
            kind: kind.into(),
            payload: payload.into(),
            created_at: Instant::now(),
            attempts: 0,
        };

        self.queued
            .entry(job.kind.clone())
            .or_insert_with(VecDeque::new)
            .push_back(job);

        id
    }

    /// Enqueue a job with the default kind.
    pub fn enqueue_default<P: Into<String>>(&mut self, payload: P) -> u64 {
        self.enqueue(self.default_kind.clone(), payload)
    }

    /// Dequeue a job with a lease. Returns None if no work available.
    pub fn dequeue(&mut self, kind: &str, lease_duration: Duration) -> Option<Job> {
        let queue = self.queued.get_mut(kind)?;
        let mut job = queue.pop_front()?;

        job.attempts = job.attempts.saturating_add(1);

        let lease = Lease {
            job: job.clone(),
            expires_at: Instant::now() + lease_duration,
        };
        self.leased.insert(job.id, lease);

        Some(job)
    }

    /// Mark a job as done; removes from leased.
    pub fn complete(&mut self, job_id: u64) -> Result<()> {
        if let Some(lease) = self.leased.remove(&job_id) {
            self.done.push(lease.job);
            Ok(())
        } else {
            Err(format!("complete(): job {job_id} not leased/unknown").into())
        }
    }

    /// Mark a job as failed; removes from leased and re-enqueues.
    pub fn fail(&mut self, job_id: u64) -> Result<()> {
    if let Some(lease) = self.leased.remove(&job_id) {
            let kind = lease.job.kind.clone();
            self.queued
                .entry(kind)
                .or_insert_with(VecDeque::new)
                .push_back(lease.job.clone());

            self.failed.push(lease.job);
            Ok(())
        } else {
            Err(format!("fail(): job {job_id} not leased/unknown").into())
        }
    }

    /// Move expired leases back to their queues (retry).
    pub fn reclaim_expired(&mut self) {
        let now = Instant::now();
        let expired_ids: Vec<u64> = self
            .leased
            .iter()
            .filter_map(|(&id, lease)| (lease.expires_at <= now).then_some(id))
            .collect();

        for id in expired_ids {
            if let Some(lease) = self.leased.remove(&id) {
                self.queued
                    .entry(lease.job.kind.clone())
                    .or_insert_with(VecDeque::new)
                    .push_back(lease.job);
            }
        }
    }

    /// Queue depth across all kinds (visible work only).
    pub fn depth(&self) -> usize {
        self.queued.values().map(|q| q.len()).sum()
    }

    /// Number of leased (in-flight) jobs.
    pub fn leased_count(&self) -> usize {
        self.leased.len()
    }          
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn test_enqueue_dequeue() {
        let mut sched = Scheduler::new();
        let job_id = sched.enqueue("email", "Send welcome email");
        assert_eq!(job_id, 1);
        assert_eq!(sched.depth(), 1);

        let job = sched.dequeue("email", Duration::from_secs(5)).unwrap();
        assert_eq!(job.id, job_id);
        assert_eq!(job.payload, "Send welcome email");
        assert_eq!(sched.depth(), 0);
        assert_eq!(sched.leased_count(), 1);
    }

    #[test]
    fn test_complete() {
        let mut sched = Scheduler::new();
        let job_id = sched.enqueue("email", "Send welcome email");
        let job = sched.dequeue("email", Duration::from_secs(5)).unwrap();
        assert_eq!(job.id, job_id);

        sched.complete(job_id).unwrap();
        assert_eq!(sched.leased_count(), 0);
        assert_eq!(sched.done.len(), 1);
    }

    #[test]
    fn test_fail_and_reenqueue() {
        let mut sched = Scheduler::new();
        let job_id = sched.enqueue("email", "Send welcome email");
        let job = sched.dequeue("email", Duration::from_secs(1)).unwrap();
        assert_eq!(job.id, job_id);

        sched.fail(job_id).unwrap();
        assert_eq!(sched.leased_count(), 0);
        assert_eq!(sched.failed.len(), 1);
        assert_eq!(sched.depth(), 1);

        // Dequeue again to check attempts incremented
        let job2 = sched.dequeue("email", Duration::from_secs(1)).unwrap();
        assert_eq!(job2.id, job_id);
        assert_eq!(job2.attempts, 2);
    }

    #[test]
    fn test_reclaim_expired() {
        let mut sched = Scheduler::new();
        let job_id = sched.enqueue("email", "Send welcome email");
        let job = sched.dequeue("email", Duration::from_millis(50)).unwrap();
        assert_eq!(job.id, job_id);

        sleep(Duration::from_millis(70)); // allow lease to expire
        sched.reclaim_expired();
        assert_eq!(sched.leased_count(), 0);
        assert_eq!(sched.depth(), 1);

        let job2 = sched.dequeue("email", Duration::from_secs(1)).unwrap();
        assert_eq!(job2.id, job_id);
        assert_eq!(job2.attempts, 2);
    }
}
