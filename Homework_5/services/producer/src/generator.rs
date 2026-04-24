use crate::config::Config;
use crate::event::{DeviceType, EventType, MovieEventPayload};
use crate::publisher::Publisher;
use chrono::Utc;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use std::sync::Arc;
use tracing::warn;

#[derive(Clone, Copy)]
enum SessionState {
    Start,
    Paused { progress: i32 },
    Running { progress: i32 },
    Done,
}

struct WatchSession {
    user_id: String,
    session_id: uuid::Uuid,
    movie_id: String,
    device: DeviceType,
    target: i32,
    state: SessionState,
}

impl WatchSession {
    fn new(rng: &mut impl Rng) -> Self {
        Self {
            user_id: format!("user-{}", rng.gen_range(1..8000)),
            session_id: uuid::Uuid::new_v4(),
            movie_id: format!("movie-{}", rng.gen_range(1..400)),
            device: random_device(rng),
            target: rng.gen_range(600..4800),
            state: SessionState::Start,
        }
    }

    fn next_event(&mut self, rng: &mut impl Rng) -> Option<MovieEventPayload> {
        let ts = Utc::now();
        let mk = |et: EventType, prog: i32| MovieEventPayload {
            event_id: uuid::Uuid::new_v4(),
            user_id: self.user_id.clone(),
            movie_id: self.movie_id.clone(),
            event_type: et,
            timestamp: ts,
            device_type: self.device,
            session_id: self.session_id.to_string(),
            progress_seconds: prog,
        };

        match self.state {
            SessionState::Start => {
                self.state = SessionState::Running { progress: 0 };
                Some(mk(EventType::ViewStarted, 0))
            }
            SessionState::Running { progress } => {
                if progress >= self.target {
                    self.state = SessionState::Done;
                    Some(mk(EventType::ViewFinished, self.target))
                } else {
                    self.state = SessionState::Paused { progress };
                    Some(mk(EventType::ViewPaused, progress))
                }
            }
            SessionState::Paused { progress } => {
                let chunk = rng.gen_range(25..200);
                let new_p = (progress + chunk).min(self.target);
                self.state = SessionState::Running { progress: new_p };
                Some(mk(EventType::ViewResumed, new_p))
            }
            SessionState::Done => None,
        }
    }
}

fn random_device(rng: &mut impl Rng) -> DeviceType {
    *[
        DeviceType::Mobile,
        DeviceType::Desktop,
        DeviceType::Tv,
        DeviceType::Tablet,
    ]
    .choose(rng)
    .unwrap()
}

async fn emit_side_event(publisher: &Arc<Publisher>, rng: &mut impl Rng) {
    let user = format!("user-{}", rng.gen_range(1..8000));
    let session_id = uuid::Uuid::new_v4().to_string();
    let ts = Utc::now();
    let device = random_device(rng);

    let payload = if rng.gen_bool(0.55) {
        MovieEventPayload {
            event_id: uuid::Uuid::new_v4(),
            user_id: user,
            movie_id: format!("movie-{}", rng.gen_range(1..400)),
            event_type: EventType::Liked,
            timestamp: ts,
            device_type: device,
            session_id,
            progress_seconds: 0,
        }
    } else {
        let queries = [
            "search:thriller",
            "search:comedy",
            "search:sci-fi",
            "search:documentary",
        ];
        MovieEventPayload {
            event_id: uuid::Uuid::new_v4(),
            user_id: user,
            movie_id: queries[rng.gen_range(0..queries.len())].to_string(),
            event_type: EventType::Searched,
            timestamp: ts,
            device_type: device,
            session_id,
            progress_seconds: 0,
        }
    };

    if let Err(e) = publisher.publish(&payload).await {
        warn!(error = %e, "generator side-event publish failed");
    }
}

pub async fn run(cfg: &Config, publisher: Arc<Publisher>) {
    let mut rng = StdRng::from_entropy();
    let mut sessions: Vec<WatchSession> = Vec::new();

    loop {
        tokio::time::sleep(cfg.generator_interval).await;

        if sessions.len() < 12 && rng.gen_bool(0.25) {
            sessions.push(WatchSession::new(&mut rng));
        }

        if rng.gen_bool(0.12) {
            emit_side_event(&publisher, &mut rng).await;
        }

        if sessions.is_empty() {
            continue;
        }

        let idx = rng.gen_range(0..sessions.len());
        match sessions[idx].next_event(&mut rng) {
            Some(ev) => {
                if let Err(e) = publisher.publish(&ev).await {
                    warn!(error = %e, "generator watch publish failed");
                }
            }
            None => {
                sessions.swap_remove(idx);
            }
        }
    }
}
