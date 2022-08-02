# Labyrinth

This is an application that can be flashed and played on the BBC micro:bit microcomputer. It utilizes the accelerometer and LEDs to indicate your location in the maze. The blinking light is your current location. Tilt the micro:bit up, down, left, or right and the dot will attempt to move in that direction.
The walls of the maze are invisible. If the direction you have chosen is blocked by an invisible wall, the dot cannot move in that direction. You will have to find a way around it!

Navigate the blinking light to the bottom right corner to pass the level!

Note: This is still very much a work in progress. The wall generation is "random" based on the raw accelerometer data. There is also no validation that there is a solution to the maze, as all valid moves may be walled off.
