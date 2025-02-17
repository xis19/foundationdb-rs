use foundationdb::{options, tuple::Subspace};
use foundationdb_simulation::{
    details, fdb_spawn, Metric, Promise, RustWorkload, Severity, SimDatabase, WorkloadContext,
};

pub struct AtomicWorkload {
    context: WorkloadContext,
    client_id: usize,
    // how many transactions will be run
    expected_count: usize,
    // how many transactions succeeded
    success_count: usize,
    // how many transactions failed
    error_count: usize,
    // how many maybe_committed transactions we encountered
    maybe_committed_count: usize,
}

impl AtomicWorkload {
    pub fn new(context: WorkloadContext) -> Self {
        Self {
            client_id: context.client_id(),
            expected_count: context.get_option("count").expect("Could not get count"),
            context,
            success_count: 0,
            error_count: 0,
            maybe_committed_count: 0,
        }
    }
}

const COUNT_KEY: &[u8] = b"count";

impl RustWorkload for AtomicWorkload {
    fn description(&self) -> String {
        "Atomic Rust Workload".into()
    }
    fn setup(&'static mut self, _db: SimDatabase, done: Promise) {
        println!("rust_setup({})", self.client_id);
        done.send(true);
    }
    fn start(&'static mut self, db: SimDatabase, done: Promise) {
        println!("rust_start({})", self.client_id);
        fdb_spawn(async move {
            // Only use a single client
            if self.client_id == 0 {
                for _ in 0..self.expected_count {
                    let trx = db.create_trx().expect("Could not create transaction");
                    let buf: [u8; 8] = 1i64.to_le_bytes();

                    trx.atomic_op(
                        &Subspace::all().pack(&COUNT_KEY),
                        &buf,
                        options::MutationType::Add,
                    );

                    match trx.commit().await {
                        Ok(_) => self.success_count += 1,
                        Err(err) => {
                            if err.is_maybe_committed() {
                                self.context.trace(
                                    Severity::Info,
                                    "Detected an maybe_committed transactions",
                                    details![
                                        "Layer" => "Rust",
                                        "Client" => self.client_id
                                    ],
                                );
                                self.maybe_committed_count += 1;
                            } else {
                                self.error_count += 1;
                            }
                        }
                    }

                    self.context.trace(
                        Severity::Info,
                        "Successfully setup workload",
                        details![
                            "Layer" => "Rust",
                            "Client" => self.client_id
                        ],
                    );
                }
            }
            done.send(true);
        });
    }
    fn check(&'static mut self, db: SimDatabase, done: Promise) {
        println!("rust_check({})", self.client_id);
        fdb_spawn(async move {
            if self.client_id == 0 {
                let trx = db.create_trx().expect("Could not create transaction");

                match trx.get(&Subspace::all().pack(&COUNT_KEY), true).await {
                    Ok(Some(fdb_slice)) => {
                        let count = i64::from_le_bytes(fdb_slice[..8].try_into().unwrap());
                        let count = count as usize;
                        // We don't know how much maybe_committed transactions has succeeded,
                        // so we are checking the possible  range
                        if self.success_count <= count
                            && count <= self.expected_count + self.maybe_committed_count
                        {
                            self.context.trace(
                                Severity::Info,
                                "Atomic count match",
                                details![
                                    "Layer" => "Rust",
                                    "Client" => self.client_id,
                                    "Expected" => self.expected_count,
                                    "Found" => count,
                                    "CommittedCount" => self.success_count,
                                    "MaybeCommitted" => self.maybe_committed_count,
                                ],
                            );
                        } else {
                            self.context.trace(
                                Severity::Error,
                                "Atomic count doesn't match",
                                details![
                                    "Layer" => "Rust",
                                    "Client" => self.client_id,
                                    "Expected" => self.expected_count,
                                    "Found" => count,
                                    "CommittedCount" => self.success_count,
                                    "MaybeCommitted" => self.maybe_committed_count,
                                ],
                            );
                        }
                    }
                    _ => {
                        self.context.trace(
                            Severity::Error,
                            "Could not get Atomic count",
                            details![
                                "Layer" => "Rust",
                                "Client" => self.client_id
                            ],
                        );
                    }
                }
            }
            done.send(true);
        });
    }
    fn get_metrics(&self) -> Vec<Metric> {
        println!("rust_get_metrics({})", self.client_id);
        vec![
            Metric::val("expected_count", self.expected_count as f64),
            Metric::val("success_count", self.success_count as f64),
            Metric::val("error_count", self.error_count as f64),
        ]
    }
    fn get_check_timeout(&self) -> f64 {
        println!("rust_get_check_timeout({})", self.client_id);
        5000.0
    }
}
