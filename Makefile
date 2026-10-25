build:
	cargo build --release

local_server:
	cargo run --release --bin server_app 5000 $$(pwd)/saves/ scenario_a

mp_client:
	cargo run --release -- -a 127.0.0.1:5000 --username mattmarti

singleplayer:
	cargo run --release -- --run-server --username kim_mcbudget

multiplayer:
	cargo run --release -- --run-server --username kim_mcbudget --use-mp-backend

generate_worlds:
	cargo run --release --bin generate_worlds
