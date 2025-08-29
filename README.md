#  CFD Dome Seeing & Wind Loads PSFs generator

Application to generate GMT PSFs under dome seeing and wind loads pertubations.

To compile the applications several environment variables need to be defined:
 - CUDACXX : CUDA nvcc compiler full command path
 - FEM_REPO : path to the GMT FEM repository that holds the windloads data files
 - CFD_REPO : path to the repository where all the CFD cases are saved
 - GMT_MODES_PATH : path to CEO mirror modes (.ceo) data fils

For later use, export all the environment variables into a `.setup` file and source them with `. .setup`.
`.gitignore` is configured to ignore `.setup` files.

There are 2 applications available: a [cli](cli/README.md) application and a [Web](web/README.md) application.

