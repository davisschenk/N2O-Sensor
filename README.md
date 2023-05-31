# N2O Raspberry Pi Data Logger
This script is used to log data from the LGR Isotopic N2O Analyzer alongside GPS data using a raspberry PI

# Basic Use
All that needs to be done when using the raspberry PI is to plug in the GPS and serial connector for the sensor and start/restart the raspberry pi. 

Once this is done data should start being logged to the thumb drive

## Data
All data is output to the directory specified by `DATA_DIRECTORY`

Files are csv's with all sensor and GPS data included, when the gps does not have a fix values will be empty

## Logging
Logs for the script can be viewed by running this command, level of loggin can be adjusted with`RUST_LOG` environment variable.

```bash
journalctl -u n2osensor.service
```

## Configuration
All configs can be found in the .env, this lives in the home directory of the pi, the configuration used is contained inside this directory as well.

## Building
The ./run script included in this project cross compiles the program for the raspberry pi and copies it over

## Service
The service is called `n2osensor.service`, this can be edited using `systemctl`.
