# syntax=docker/dockerfile:1
# sdkwork-knowledgebase standalone container image.
# runtimeTarget = "container", deploymentProfile = "standalone".
# Build context: an unpacked container install package directory assembled by
# scripts/build-knowledgebase-container.mjs (dist/container-image-build)
# containing bin/ (gateway + worker release binaries), portal/dist,
# database-modules/, agent.app.config.json, container/entrypoint and
# install-manifest.json.
#
# The committed copy is the build input for `pnpm build:container`; it is
# equivalent to the container/Containerfile generated inside install packages.

FROM debian:bookworm-slim

ARG GATEWAY_BINARY=sdkwork-api-knowledgebase-standalone-gateway
ARG WORKER_BINARY=sdkwork-knowledgebase-worker
ARG INSTALL_ROOT=/opt/sdkwork/knowledgebase
ARG VERSION=0.0.0

# Runtime directory layout (RUNTIME_DIRECTORY_SPEC §4.5 Container Scope):
# config mounts at /etc/sdkwork/knowledgebase, secrets at
# /run/secrets/sdkwork/knowledgebase, durable data at
# /var/lib/sdkwork/knowledgebase, cache at /var/cache/sdkwork/knowledgebase.
# libssl3 and ca-certificates are runtime dependencies of the gateway/worker
# binaries (PostgreSQL TLS and outbound HTTPS); the slim base image does not
# carry them. curl powers the container healthchecks and operational
# diagnostics.
RUN apt-get update \
  && apt-get install -y --no-install-recommends libssl3 ca-certificates curl \
  && rm -rf /var/lib/apt/lists/* \
  && groupadd --system sdkwork \
  && useradd --system --gid sdkwork --home-dir ${INSTALL_ROOT} sdkwork \
  && mkdir -p ${INSTALL_ROOT} /etc/sdkwork/knowledgebase /run/sdkwork/knowledgebase \
    /var/lib/sdkwork/knowledgebase /var/cache/sdkwork/knowledgebase \
    /var/log/sdkwork/knowledgebase \
  && chown -R sdkwork:sdkwork /etc/sdkwork/knowledgebase /run/sdkwork/knowledgebase \
    /var/lib/sdkwork/knowledgebase /var/cache/sdkwork/knowledgebase \
    /var/log/sdkwork/knowledgebase

WORKDIR ${INSTALL_ROOT}
COPY . ${INSTALL_ROOT}
RUN chmod 0755 ${INSTALL_ROOT}/bin/${GATEWAY_BINARY} \
    ${INSTALL_ROOT}/bin/${WORKER_BINARY} \
    ${INSTALL_ROOT}/container/entrypoint \
  && chown -R sdkwork:sdkwork ${INSTALL_ROOT}/database-modules

ENV SDKWORK_KNOWLEDGEBASE_DEPLOYMENT_PROFILE=standalone
ENV SDKWORK_KNOWLEDGEBASE_RUNTIME_TARGET=container
# Database module roots: compile-time app roots do not exist inside the image,
# so each database host resolves its packaged module under
# <install root>/database-modules/<workspace> (the module root itself is the
# app root env value; DefaultDatabaseModule reads <app root>/database).
ENV SDKWORK_KNOWLEDGEBASE_APP_ROOT=${INSTALL_ROOT}/database-modules/sdkwork-knowledgebase \
    SDKWORK_IAM_APP_ROOT=${INSTALL_ROOT}/database-modules/sdkwork-iam \
    SDKWORK_DRIVE_APP_ROOT=${INSTALL_ROOT}/database-modules/sdkwork-drive \
    SDKWORK_WEB_STORE_APP_ROOT=${INSTALL_ROOT}/database-modules/sdkwork-web-framework
# Application identity root: agent.app.config.json is installed at the
# install root; IAM tenant provisioning resolves it via SDKWORK_APP_ROOT.
ENV SDKWORK_APP_ROOT=${INSTALL_ROOT}
# Operator binaries must be on PATH so `docker compose exec` diagnostics work
# without absolute paths.
ENV PATH=${INSTALL_ROOT}/bin:${PATH}

LABEL org.opencontainers.image.title="sdkwork-knowledgebase (standalone container)"
LABEL org.opencontainers.image.version="${VERSION}"
LABEL org.opencontainers.image.vendor="sdkwork"

USER sdkwork
EXPOSE 18081
ENTRYPOINT ["/opt/sdkwork/knowledgebase/bin/sdkwork-api-knowledgebase-standalone-gateway"]
