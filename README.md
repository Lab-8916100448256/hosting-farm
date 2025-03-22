# Hosting Farm
A web application to manage a farm of self-hosting servers


## Developement dependencies
Hosting Farm is developed in Rust with the Loco-rs framework

This section describes how to install the developement dependencies of the project on a Debian GNU/Linux computer. These instructions should also work for debian based Linux distribution like Ubuntu.

### Installing git 
```
sudo apt update && sudo apt install git -y
git config --global user.name "Your Name"
git config --global user.email "you-email@example.com",
```

### Installing Rust
```
sudo apt update && sudo apt install build-essential -y
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup component add rust-analyzer rust-src
```


### Installing loco-rs and sea-orm CLI tools
```
sudo apt update && sudo apt install pkg-config -y
cargo install loco
cargo install sea-orm-cli
cargo install sea-orm-cli
```

## Building the application  
To build the application, run the following command :
```  
cargo build
```  

## Running automated tests  
To run unit tests, documentation tests and integration tests run the following command :  
```
cargo test
```  
It will also build the application if it has not been already built or if the source code has changed since the last build.


## Starting a development instance of the application
To run a development instance of the application, run the following command :
```
cargo loco start
```
It will also build the application if it has not been already built or if the source code has changed since the last build.

If building the application was successfull, a development instance should be listening on http://localhost:5150




