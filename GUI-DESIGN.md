# Qt-like GUI Library Design for Still

## Overview
A widget-based GUI library built on top of the existing gui.strc command model.
Inspired by Qt's widget hierarchy and signal/slot mechanism.

## Core Concepts

### Widget Hierarchy
```
Widget (base)
├── Container
│   ├── Window
│   ├── Panel
│   ├── GroupBox
│   └── TabWidget
├── Button
├── Label
├── LineEdit
├── TextEdit
├── ComboBox
├── CheckBox
├── RadioButton
├── Slider
├── ProgressBar
├── ListView
├── TreeView
└── Canvas (for custom drawing)
```

### Layout System
- QVBoxLayout: vertical box layout
- QHBoxLayout: horizontal box layout
- QGridLayout: grid layout
- QFormLayout: form layout (label + widget pairs)

### Signal/Slot (using closures)
```
button.clicked.connect(func() -> none:
    label.text = "Clicked!"
)
```

### Styling
- CSS-like stylesheet support
- Theme system (dark/light)
- Custom widget painting

## Implementation Plan
1. Design document (this file) ✅
2. Base Widget type with geometry, visibility, parent/child
3. Container widgets (Window, Panel)
4. Basic input widgets (Button, Label, LineEdit)
5. Layout managers
6. Signal/slot mechanism (requires closures - done!)
7. Styling system
8. Canvas for custom drawing
9. Example applications

## Still Integration
- Uses `@soa` for widget collections
- Uses closures for signal handlers
- Uses enums for widget types
- Uses traits for widget interfaces (Drawable, Clickable, etc.)

## Example Usage
```
use gui

window = Window("My App", 800, 600)
layout = QVBoxLayout()

label = Label("Hello, World!")
button = Button("Click me")
button.clicked.connect(func() -> none:
    label.text = "Button clicked!"
)

layout.add(label)
layout.add(button)
window.set_layout(layout)
window.show()
```