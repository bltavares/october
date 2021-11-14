# October

Wake-on-lan webapp - 🎵 Wake me up, when september ends 🎵

## Usage

To start, run the self-contained binary with a CSV list of `host,mac` and visit [localhost:3493](http://localhost:3493).
It should render a table of devices using a default template.

```sh
october -a sample.csv
```

### Overriding the template

You can override the internal template with the `--template` flag, giving it a [handlebar](https://handlebarsjs.com/guide/) template file.
For faster development, you can use the `--autoreload` flag to preview changes locally.

The service will host the current working directory as HTTP assets if you want to provide images or CSS styling.

## Run

This project aims to run on Docker with minimal size, and also targets multiplaform builds, including Windows, Linux and Raspberry Pi (Zero to 4).

You can either compile it with `cargo build --release` or use the provided `bltavares/october` Docker image.

To run properly, it requires to be on the `host` network without any isolation, otherwhise your router will not advertise the 'magic packet' to your devices.

### Targets

| Platform         | Docker | Size   |
|------------------|--------|--------|
| armv7-musleabihf | Yes    | 2.03MB |
| arm-musleabi     | Yes    | 2.08MB |
| aarch64-musl     | Yes    | 1.99MB |
| x86_64-musl      | Yes    | 2.49MB |
| Windows          | No     | 2.29MB |
| Mac              | No     | N/A    |

### Docker

Example command for Docker

```shell
docker run -d \
  --restart=unless-stopped \
  --network=host \
  -p 3493:3493 \
  -v sample.csv:/opt/sample.csv \
  --name october \
  bltavares/october -a /opt/sample.csv
```

## Build

To build and publish multi-architecture docker images:

```shell
make all
make publish
make manifest
```

## Debugging

> sudo netcat -ulp 9

Then you should see something from netcat.
