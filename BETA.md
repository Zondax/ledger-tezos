# Beta Testing


This document describes the procedure required to install and use this application on a ledger Nano S device.

# DISCLAIMER

*THIS IS AN UNVETTED DEVELOPMENT RELEASE*

> # This app is still work in progress!
> # DO NOT USE YET!
>
> -------------------
>
> ![zondax](docs/zondax.jpg)
>
>_Please visit our website at [zondax.ch](zondax.ch)_

> # PROCEED AT YOUR OWN RISK!
> # THESE ARE UNVETTED DEVELOPMENT RELEASES

# Prerequisites

This is the only configuration tested during development, any variation is completely new and may cause unexpected things to happen.
No warranty is provided, we do not assume any responsibility for the use or misuse of the project.
You may [spontaneously combust](https://en.wikipedia.org/wiki/Spontaneous_human_combustion), without us taking any responsibility.

* A linux machine
* python 3
* ledgerblue pip package
* A ledger Nano S device

Note: Nano X device is not currently supported for _development_ & _testing_, an official app will be released for Nano X devices by Ledger.

# Preparation

Download the [latest release](https://github.com/Zondax/ledger-tezos/releases)'s `installer_s.sh` or `installer_baking_s.sh`, depending if you'd like to use the "wallet" or "baking" app.

Once downloaded, mark the file as executable with
```sh
chmod +x ./installer_s.sh
```

# Installation

*MAKE SURE YOUR NANO S DEVICE IS CONNECTED TO YOUR MACHINE*

You can now run the installation script with the `load` argument to install the app on your device:
```sh
./installer_sh.sh load
```

# Feedback

You can reach out to us on Slack or you can open issues on the [official GitHub repository](https://github.com/Zondax/ledger-tezos/), for any feedback or further questions.

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
