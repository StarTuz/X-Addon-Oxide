import struct
import sys
import os

def create_icns(iconset_dir, output_file):
    # Mapping of ICNS types to filenames in the iconset directory
    types = {
        b'icp4': 'icon_16x16.png',
        b'icp5': 'icon_32x32.png',
        b'icp6': 'icon_64x64.png',
        b'ic07': 'icon_128x128.png',
        b'ic08': 'icon_256x256.png',
        b'ic09': 'icon_512x512.png',
        b'ic10': 'icon_1024x1024.png',
        # Retinas/High res
        b'ic11': 'icon_16x16@2x.png',
        b'ic12': 'icon_32x32@2x.png',
        b'ic13': 'icon_128x128@2x.png',
        b'ic14': 'icon_256x256@2x.png',
    }
    
    data_blocks = []
    for tag, filename in types.items():
        path = os.path.join(iconset_dir, filename)
        # Handle some variants
        if not os.path.exists(path):
            if tag == b'icp4': path = os.path.join(iconset_dir, 'icon_16x16.png')
            if tag == b'icp5': path = os.path.join(iconset_dir, 'icon_32x32.png')
            # ... add more fallbacks if needed
            
        if os.path.exists(path):
            with open(path, 'rb') as f:
                img_data = f.read()
                # Block = 4 bytes tag, 4 bytes size (big endian, includes header), data
                block_size = len(img_data) + 8
                data_blocks.append(tag + struct.pack('>I', block_size) + img_data)
        else:
            print(f"Warning: {filename} not found, skipping {tag.decode()}")

    if not data_blocks:
        print("Error: No icons found to pack!")
        sys.exit(1)

    # File = 'icns' header, 4 bytes total size (big endian, includes header), data_blocks
    total_size = sum(len(b) for b in data_blocks) + 8
    with open(output_file, 'wb') as f:
        f.write(b'icns' + struct.pack('>I', total_size))
        for block in data_blocks:
            f.write(block)
    
    print(f"Successfully created {output_file}")

if __name__ == "__main__":
    if len(sys.argv) < 3:
        print("Usage: python3 make_icns.py <iconset_dir> <output_file>")
        sys.exit(1)
    create_icns(sys.argv[1], sys.argv[2])
