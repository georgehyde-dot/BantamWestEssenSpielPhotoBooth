# Selection Image Requirements

## Overview
This document outlines the image requirements for the photo booth selection pages (weapon, land, and companion selection screens).

## Image Locations

All selection images should be placed in the following directory:
```
/static/resized_output/
```

**Note:** You will need to create the `resized_output` directory inside the `static` folder if it doesn't exist.

## Image Files Required

### Weapon Selection Page
- `sword.jpg` - Button 1 (top-left)
- `hammer.jpg` - Button 2 (top-right)
- `pistol.jpg` - Button 3 (bottom-left)
- `fie.jpg` - Button 4 (bottom-right)

### Land Selection Page
- `hill.jpg` - Button 1 (top-left)
- `house.jpg` - Button 2 (top-right)
- `road.jpg` - Button 3 (bottom-left)
- `stream.jpg` - Button 4 (bottom-right)

### Companion Selection Page
- `cat.jpg` - Button 1 (top-left)
- `dog.jpg` - Button 2 (top-right)
- `duck.jpg` - Button 3 (bottom-left)
- `horse.jpg` - Button 4 (bottom-right)

## Image Dimensions

### Recommended Size
- **Optimal:** 400x400 pixels
- **Minimum:** 200x200 pixels
- **Maximum:** 600x600 pixels

### Aspect Ratio
- **Required:** 1:1 (square)
- Images will be displayed with `object-fit: cover`, so non-square images will be cropped

## File Format Requirements
- **Format:** JPEG (.jpg)
- **Color Mode:** RGB
- **Quality:** 80-90% JPEG quality recommended for balance between file size and visual quality
- **File Size:** Keep under 500KB per image for optimal loading performance

## Display Specifications

### Button Display Sizes
- **Default screens:** 200x200 pixels
- **Small screens (height < 600px):** 150x150 pixels
- **Responsive scaling:** Buttons scale to 25% of viewport width (max 200px)

### Layout
- 2x2 grid layout (4 buttons total per page)
- 15px gap between buttons
- 10px rounded corners on buttons

## Image Preparation Tips

1. **Cropping:** Ensure main subject is centered as images will be displayed as squares
2. **Contrast:** Use high contrast images that are clearly visible against the dark background (#2c3e50)
3. **Consistency:** Try to maintain similar visual style/theme across all images
4. **Testing:** Test images at different screen sizes to ensure they look good when scaled

## Directory Structure Example
```
canon_test_cam/
└── static/
    ├── background.png (existing template background)
    └── resized_output/
        ├── sword.jpg
        ├── hammer.jpg
        ├── pistol.jpg
        ├── fie.jpg
        ├── hill.jpg
        ├── house.jpg
        ├── road.jpg
        ├── stream.jpg
        ├── cat.jpg
        ├── dog.jpg
        ├── duck.jpg
        └── horse.jpg
```

## Notes
- The HTML files reference these images with the path `/static/resized_output/[filename].jpg`
- The images are served by the Actix web server configured in `main.rs`
- If images need to be changed, simply replace the files in the `resized_output` directory with the same filenames