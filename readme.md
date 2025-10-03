# paintelf

Small utility to extract data files from Paper Mario: Color Splash

## Usage

Make sure you have a ROM dump of Paper Mario: Color Splash. Decompress the .elf file using something like [KillzXGaming's Switch Toolbox](https://github.com/KillzXGaming/Switch-Toolbox) (in the toolbar at the top, go to Tools > Compression > LZ77 > Decompress).

Currently, the only supported file is data_fld_maplink.elf.

On Windows, you can extract the file into a text format by dragging the file onto the paintelf.exe file. Alternatively, you can do it like this:

    paintelf <path to .elf>

It will create another file right next to it with the same name but ending on .yaml.
