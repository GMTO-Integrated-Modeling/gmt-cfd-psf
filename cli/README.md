#  CFD Dome Seeing & Wind Loads PSFs generator from the terminal

The application needs several environment variables to be defined:
 - CUDACXX : CUDA nvcc compiler full command path
 - FEM : path to the GMT FEM repository that holds the windloads data files
 - CFD_REPO : path to the repository where all the CFD cases are saved
 - GMT_MODES_PATH : path to CEO mirror modes (.ceo) data fils

The environment variables must be written into a `.env` file.

If uploading the data remotely from AWS S3, the environment variables must be written into a `.env_s3` file and in addition to the variable above, the following ones need to be provided as well:
 - REGION : the AWS region where the S3 bucket resides
 - BUCKET : the S3 bucket name
 - AWS_ACCESS_KEY_ID and AWS_SECRET_ACCESS_KEY : AWS credentials

## Usage

```shell
cargo r -r -- --help
```

Dome seeing PSF:

```shell
cargo r -r -- --domeseeing
```

Windloads PSF:

```shell
cargo r -r -- --windloads
```


