APPLICATION_NAME ?= yttopocketcasts

run:
		$(MAKE) _builder
		$(MAKE) _runner

_builder:
		docker build --tag ${APPLICATION_NAME} .

_runner:
		docker run --rm ${APPLICATION_NAME}