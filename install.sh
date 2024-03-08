#!/bin/bash
# print the splash ascii art only if a screenreader is not detected. TODO detect screenreaders that aren't orca?
if ! command -v orca &> /dev/null
then
  echo $'     / \\ / \\ / \\ \n    |   |   |   |                     _ \n   / \\ \e[31m* * * *\e[0m / \\                   | | \n  |   \e[31m*   *   *\e[0m   |        __ _ _   _| |_ ___  _ __ ___   __ _ _ __   ___ _   _ \n / \\ \e[31m* * \e[0m/ \\\e[31m * *\e[0m / \\      / _` | | | | __/ _ \\| `_ ` _ \\ / _` | `_ \\ / __| | | | \n|   \e[31m*\e[0m   |   |   \e[31m*\e[0m   |    | (_| | |_| | || (_) | | | | | | (_| | | | | (__| |_| | \n \\ / \e[31m* *\e[0m \\ / \e[31m* * *\e[0m /      \\__,_|\\__,_|\\__\\___/|_| |_| |_|\\__,_|_| |_|\\___|\\__, | \n  |   \e[31m*   *   *   *\e[0m                                                        __/ | \n   \\ / \e[31m* * * * * *\e[0m                                                        |___/ \n    |   |   |   |\n     \\ / \\ / \\ / \n'
else
  echo "Welcome to the automancy installer. A screenreader has been detected on your system; you may want to quit this installer and install automancy-nv instead."
fi
# check whether the automancy binary and the resources folder exist, otherwise exit
echo "Checking for files..."
if ! [ -f "./automancy" ] || ! [ -d "./resources" ]
then
  echo $'\e[1;31mCould not find all of the installation files! Please try redownloading.\e[0m'
  exit 1
fi
