# The Database Generator
The database generator will create a database binary based on the architecture explained [here](database_architecture.md).
Throughout this documentation ⚙️ `some_setting` is used to indicate that something is configurable via the [config_file](../config/config.toml).

## 0. High level overview
TODO: explain checkpoints, explain corruption through the change of settings during the generation phase

## 1. The Generator Pipeline

### 1.1 Data Download
The database generator downloads the necessary wikidata dumps from [wikimedia](https://dumps.wikimedia.org) and the actual content from a mirror provided by [Friedrich-Alexander-Universität Erlangen-Nürnburg](https://ftp.fau.de/). You can change the downloaded source in the [config_file file](../config/config.toml) via the ⚙️ `*_url` settings.

### 1.2 


## 2. Adding your own Article Processing Function
