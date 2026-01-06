cargo run -p mpc-backend-mock -- --config config.yaml run

cargo build --bin mpc-backend-mock

cargo sqlx mig run --ignore-missing
cargo sqlx mig revert --ignore-missing
