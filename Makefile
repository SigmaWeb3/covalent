build-contracts:
	cd ckb-contracts && capsule build --release

build-contracts-debug:
	cd ckb-contracts && capsule build

schema:
	make -C types schema
