obj-m += headers.o

all:
	make -C /lib/modules/$(shell uname -r)/build M=$(PWD) modules V=1 | tee build_output.log

clean:
	make -C /lib/modules/$(shell uname -r)/build M=$(PWD) clean
