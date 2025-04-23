use stakker::{Actor, ActorOwn, Stakker};

// Coordinator message enum
pub enum CoordinatorMsg {
    HandleTest,
}

// Coordinator actor
pub struct AppCoordinator;

impl AppCoordinator {
    // Add an init function as required by the actor! macro
    pub fn init(_: stakker::CX![]) -> Option<Self> {
        Some(Self)
    }

    // Example handler function (needs to be adapted based on actual usage)
    pub fn handle_test(&mut self, _: stakker::CX![]) {
        tracing::info!("Coordinator received TestCore message");
    }
}

pub struct CoreHandles {
    pub coordinator: ActorOwn<AppCoordinator>,
}

pub fn init_core_actors(stakker: &mut Stakker) -> CoreHandles {
    // Use the actor! macro to create the actor instance
    // Assuming no return handler is needed for now (ret_nop!)
    let coordinator = stakker::actor!(stakker, AppCoordinator::init(), stakker::ret_nop!());
    CoreHandles { coordinator }
}

// Retain stub for compatibility
pub fn core_hello() {
    tracing::info!("Core library initialized");
}
