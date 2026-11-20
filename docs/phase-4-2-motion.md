# Phase 4.2 motion and input

[LevelTransition](../experiments/spatial-alpha/LevelTransition.qml) owns each level's fade and scale. Depth navigation updates the hierarchy synchronously, then animates the incoming viewport inward or outward based on direction. The outgoing layer can finish fading after input has moved to the incoming layer. Level zoom and fade use a 240 ms ease-out. The Worlds carousel uses a 220 ms sibling movement. Control focus uses 160 ms, and the overlays use a 180 ms fade.

A new level change stops its preceding entrance before starting the next target. There are no completion callbacks that own or reconstruct navigation state. Reduced motion settles presentation state without a camera move. Losing window activation cancels any in-progress gesture.

A pointer gesture uses separate selection and activation handlers and a scene-wide double-drill guard. A short one-shot delay keeps a newly clicked peripheral World stable until the platform double-click interval ends. Ctrl-wheel steps by level through a direction latch and cooldown. Regular wheel input navigates Worlds or pans denser views. Keyboard and palette navigation update the same logical selection.

The atmosphere is static between state changes. Its inexpensive parallax follows input or level changes. There is no ambient animation clock while the scene is idle.
