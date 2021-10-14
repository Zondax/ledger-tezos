# Beta Testing

This document describes the procedure required to install and use this application on a ledger Nano S device.

# DISCLAIMER

_THIS IS AN UNVETTED DEVELOPMENT RELEASE_

> # This app is still work in progress!
>
> # DO NOT USE YET!
>
> ---
>
> ![zondax](docs/zondax.jpg)
>
> _Please visit our website at [zondax.ch](zondax.ch)_

> # PROCEED AT YOUR OWN RISK!
>
> # THESE ARE UNVETTED DEVELOPMENT RELEASES

# Prerequisites

This is the only configuration tested during development, any variation is completely new and may cause unexpected things to happen.

No warranty is provided, we do not assume any responsibility for the use or misuse of the project.

You may [spontaneously combust](https://en.wikipedia.org/wiki/Spontaneous_human_combustion), without us taking any responsibility.

- A linux machine
- python 3
- A ledger Nano S device

We also require a python package, the `ledgerblue` package.

To install, run the following:

```sh
pip install ledgerblue
```

Note: Nano X device is not currently supported for _development_ & _testing_, an official app will be released for Nano X devices by Ledger.

# Preparation

Download the [latest release](https://github.com/Zondax/ledger-tezos/releases).

Depending if you'd like to use either the "wallet" or "baking" app, you might choose to download, respectively, either of:

- `installer_s.sh` (wallet app)
- `installer_baking_s.sh` (baking app)

From here on out when you see `installer.sh` (or installer) it means the installer you chose to download.

Once downloaded, mark the installer as executable with

```sh
chmod +x ./installer.sh
```

# Installation

_MAKE SURE YOUR NANO S DEVICE IS CONNECTED TO YOUR MACHINE_

You can now run the installation script with the `load` argument to install the app on your device:

```sh
./installer.sh load
```

# Feedback

To reach out to us, please use one of the following methods:

- via Slack
- via email [support.tezos@zondax.ch](mailto:support.tezos@zondax.ch)
- via opening issues on the [official GitHub repository](https://github.com/Zondax/ledger-tezos/)

## Request for testing

We'd like to hear all feedback you have, but there's a few points we need testing and feedback on, namely:

### UI

How can the new UI be improved?

Is something missing?

Is something too much and should be hidden, perhaps behind an expert mode?

### Legacy API

Is there any incompatibilities with the old API from the legacy app?

What are these?

### Missing features

Is there any feature that is missing?

What could be improved?
