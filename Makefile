IMAGE = bltavares/october
VERSION = 0.1.0
CACHE_BUST = $(shell date -u +"%Y-%m-%dT00:00:00Z")

define build_image
  docker build . --pull \
    --tag $(IMAGE):$(1) \
	--tag $(IMAGE):$(VERSION)-$(1) \
	--build-arg VERSION=$(VERSION) \
	--build-arg BUILD_DATE=$(CACHE_BUST) \
	--build-arg BUILDER_ARCH=$(2) \
	--build-arg TARGET_ARCH=$(3)
endef

amd64:
	$(call build_image,amd64,x86_64-musl,x86_64-unknown-linux-musl)

arm64:
	$(call build_image,arm64v8,aarch64-musl,aarch64-unknown-linux-musl)

armhf:
	$(call build_image,arm32v7,armv7-musleabihf,armv7-unknown-linux-musleabihf)

armel:
	$(call build_image,arm32v5,arm-musleabi,arm-unknown-linux-musleabi)


publish:
	docker push $(IMAGE):amd64
	docker push $(IMAGE):arm32v5
	docker push $(IMAGE):arm32v7
	docker push $(IMAGE):arm64v8

	docker push $(IMAGE):$(VERSION)-amd64
	docker push $(IMAGE):$(VERSION)-arm32v5
	docker push $(IMAGE):$(VERSION)-arm32v7
	docker push $(IMAGE):$(VERSION)-arm64v8

manifest:
	docker manifest create \
	  $(IMAGE):$(VERSION) \
	  $(IMAGE):$(VERSION)-amd64 \
	  $(IMAGE):$(VERSION)-arm32v5 \
	  $(IMAGE):$(VERSION)-arm32v7 \
	  $(IMAGE):$(VERSION)-arm64v8

	docker manifest annotate $(IMAGE):$(VERSION) \
	  $(IMAGE):$(VERSION)-amd64 --os linux \
	  --arch amd64

	docker manifest annotate $(IMAGE):$(VERSION) \
	  $(IMAGE):$(VERSION)-arm32v5 --os linux \
	  --arch arm --variant v5

	docker manifest annotate $(IMAGE):$(VERSION) \
	  $(IMAGE):$(VERSION)-arm32v7 --os linux \
	  --arch arm --variant v7

	docker manifest annotate $(IMAGE):$(VERSION) \
	  $(IMAGE):$(VERSION)-arm64v8 --os linux \
	  --arch arm64

	docker manifest push --purge $(IMAGE):$(VERSION)

	docker manifest create \
	  $(IMAGE):latest \
	  $(IMAGE):amd64 \
	  $(IMAGE):arm32v5 \
	  $(IMAGE):arm32v7 \
	  $(IMAGE):arm64v8

	docker manifest annotate $(IMAGE):latest \
	  $(IMAGE):amd64 --os linux \
	  --arch amd64

	docker manifest annotate $(IMAGE):latest \
	  $(IMAGE):arm32v5 --os linux \
	  --arch arm --variant v5

	docker manifest annotate $(IMAGE):latest \
	  $(IMAGE):arm32v7 --os linux \
	  --arch arm --variant v7

	docker manifest annotate $(IMAGE):latest \
	  $(IMAGE):arm64v8 --os linux \
	  --arch arm64

	docker manifest push --purge $(IMAGE):latest

all: amd64 arm64 armel armhf

.PHONY: all arm64 armel armhf amd64 publish manifest