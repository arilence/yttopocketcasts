#!make
-include .env

APPLICATION_NAME ?= yttopocketcasts

run:
		$(MAKE) _builder
		$(MAKE) _runner

stop:
		$(MAKE) _stopper

_builder: FORCE
		docker build --tag ${APPLICATION_NAME} .
FORCE: ;

_runner:
		@docker run --rm -it \
        --name ${APPLICATION_NAME} \
        --env TELOXIDE_TOKEN="${TELOXIDE_TOKEN}" \
        --env TRUSTED_USER_IDS="${TRUSTED_USER_IDS}" \
        --env ADMIN_USER_IDS="${ADMIN_USER_IDS}" \
        ${APPLICATION_NAME} \
        /bin/bash

_stopper:
		docker stop ${APPLICATION_NAME}
