deploy:
	cargo build --release
	sudo cp target/release/server-dash-api /var/lib/server-dash-api/server-dash-api
	sudo chown server-dash-api:server-dash-api /var/lib/server-dash-api/server-dash-api
	sudo chmod 755 /var/lib/server-dash-api/server-dash-api
	sudo systemctl restart server-dash-api
