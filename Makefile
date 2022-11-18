APPLICATION_NAME ?= yttopocketcasts

run:
		$(MAKE) _builder
		$(MAKE) _runner

stop:
		$(MAKE) _stopper

_builder:
		docker build --tag ${APPLICATION_NAME} .

_runner:
		docker run --rm --name ${APPLICATION_NAME} ${APPLICATION_NAME}

_stopper:
		docker stop --time 1 ${APPLICATION_NAME}
