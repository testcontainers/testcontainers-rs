/// It allows using both runtime flavors without indefinite hanging.
/// Most useful for async calls in `Drop` implementations.
macro_rules! block_on {
    ($future:expr, $err_msg:literal) => {
        let handle = tokio::runtime::Handle::current();
        match handle.runtime_flavor() {
            RuntimeFlavor::CurrentThread => {
                // Not the best approach, but it greatly simplifies the code and use of the library.
                // since the main scenario is testing, this is acceptable.
                std::thread::spawn(move || {
                    tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap()
                        .block_on($future);
                })
                .join()
                .expect($err_msg);
            }
            RuntimeFlavor::MultiThread => {
                tokio::task::block_in_place(move || handle.block_on($future))
            }
            _ => unreachable!("unsupported runtime flavor"),
        }
    };
}

pub(crate) use block_on;
