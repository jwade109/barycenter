build:
	cargo build --release

local_server:
	cargo run --release --bin server_app 5000 $$(pwd)/saves/ scenario_a

mp_client:
	cargo run --release -- -a 127.0.0.1:5000 --username mattmarti

server:
	cargo run --release --bin server -- saves/

debugger:
	cargo run --release --bin bary_param_editor

game:
	cargo run --release --bin client_app -- --use-mp-backend --username kim_mcbudget

run:
	cargo run --release --bin program_harness
