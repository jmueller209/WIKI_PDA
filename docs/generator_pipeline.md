# The Database Generator
The database generator will create a database binary based on the architecture explained [here](database_architecture.md).
Throughout this documentation ⚙️ `some_setting` is used to indicate that something is configurable via the [config_file](../config/config.toml).

## 0. High level overview
TODO: explain checkpoints, explain corruption through the change of settings during the generation phase, explain project folder structure, temporary files, etc.

## 1. The Generator Pipeline

### 1.0 Data Download
The database generator downloads the necessary wikidata dumps from [wikimedia](https://dumps.wikimedia.org) and the actual content from a mirror provided by [Friedrich-Alexander-Universität Erlangen-Nürnburg](https://ftp.fau.de/). You can change the downloaded source in the [config_file file](../config/config.toml) via the ⚙️ `*_url` settings.

### 1.1 Parse Wiki Data

### 1.2 Compression Setup (Training ZSTD Dictionary)

### 1.3 Process Data and Create Content Binary

### 1.4 Create Metadata Binary

### 1.5 Create QID Index Binary

### 1.6 Create Binaries for Main Search Indexes

### 1.7 Merge Binaries

### 1.8 Make C Header File

## 2. Upload Database to Medium

## 3. Adding your own Article Processing Function
