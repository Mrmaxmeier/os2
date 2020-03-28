buildflavor ?= debug
drive = -drive format=raw,file=target/x86_64-unknown-elf/$(buildflavor)/bootimage-kernel.bin
memory = -m 1G
# serial = --serial mon:stdio
serial = --serial stdio
netdev = -netdev user,id=net0,hostfwd=tcp::1234-:1234 -device e1000,netdev=net0
display = -display none

# pcaps: -object filter-dump,id=f1,netdev=net0,file=dump.pcap

build:
	cargo bootimage
	cargo bootimage --release

monitor:
	qemu-system-x86_64 \
		$(memory) \
		$(serial) \
		$(drive) \
		$(netdev) \
		-nographic

run:
	qemu-system-x86_64 \
		$(memory) \
		$(serial) \
		$(drive) \
		$(netdev) \
		$(display)

debug:
	qemu-system-x86_64 \
		$(memory) \
		$(serial) \
		$(drive) \
		$(netdev) \
		$(display) \
		-S -s

record:
	qemu-system-x86_64 \
		-m 1G \
		--serial stdio \
		-drive format=raw,file=target/x86_64-unknown-elf/debug/bootimage-kernel.bin \
		-display none \
		-net none \
		-icount shift=7,rr=record,rrfile=replay.bin

replay:
	qemu-system-x86_64 \
		-m 1G \
		--serial stdio \
		-drive format=raw,file=target/x86_64-unknown-elf/debug/bootimage-kernel.bin \
		-display none \
		-net none \
		-icount shift=7,rr=replay,rrfile=replay.bin


# for use with cargo watch -x bootimage
watchrun:
	ls target/x86_64-unknown-elf/$(buildflavor)/bootimage-kernel.bin | entr -r make run buildflavor=$(buildflavor)
