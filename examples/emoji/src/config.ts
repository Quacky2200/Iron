// Numbers the stage, the curtain and the motion all agree on (world units unless noted).

/** The floor the emoji stand on. */
export const GROUND = -3.7;
/** How tall an emoji stands (the camera is 10 units tall). */
export const HEIGHT = 4.7;
/** The emoji panel's size in UI pixels. */
export const PANEL = 480;
/** The rocket's panel is bigger, so it has room to turn upright and for its flame (the picture is clipped to its panel). */
export const BIG = 768;
/** Where the spotlight lands. */
export const LIGHT = { x: 0, y: 0.2 };
/** Half the width the camera sees (it's 5 units tall), kept up to date by the director. */
export const view = { half: 8.9 };
/** How far in from its outer edge a fully drawn curtain reaches, as a share of its half. */
export const GATHER = 0.88;
