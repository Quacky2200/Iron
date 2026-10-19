"""Writes assets/sfx/*.wav for the emoji example: every sound is synthesised here in plain Python
(no samples, no libraries), so they are original and free to use (CC0).

    python tools/make_sfx.py            # all sounds
    python tools/make_sfx.py fart wah   # just these

Mono, 16-bit, 22.05 kHz. The recipes are simple (oscillators, noise, biquad filters, formant
voices); they are cartoon approximations, not recordings.
"""

import math
import os
import random
import sys
import wave

SR = 22050
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "assets", "sfx")
rng = random.Random(11)
TAU = 2 * math.pi


# ---- building blocks ----

def n_of(seconds):
    return int(seconds * SR)


def hz(midi):
    return 440.0 * 2 ** ((midi - 69) / 12)


def noise(n):
    return [rng.uniform(-1, 1) for _ in range(n)]


def coeffs(kind, f, q):
    w0 = TAU * min(max(f, 20.0), SR * 0.45) / SR
    alpha = math.sin(w0) / (2 * q)
    cw = math.cos(w0)
    if kind == "lp":
        b0, b1, b2 = (1 - cw) / 2, 1 - cw, (1 - cw) / 2
    elif kind == "hp":
        b0, b1, b2 = (1 + cw) / 2, -(1 + cw), (1 + cw) / 2
    else:  # band pass, 0 dB at the centre
        b0, b1, b2 = alpha, 0.0, -alpha
    a0 = 1 + alpha
    return b0 / a0, b1 / a0, b2 / a0, -2 * cw / a0, (1 - alpha) / a0


def filt(x, kind, f, q=0.707):
    """A biquad. `f` is a frequency in Hz or a function of the sample index (updated per block)."""
    y = [0.0] * len(x)
    z1 = z2 = 0.0
    block = 64
    for s in range(0, len(x), block):
        b0, b1, b2, a1, a2 = coeffs(kind, f(s) if callable(f) else f, q)
        for i in range(s, min(s + block, len(x))):
            v = x[i]
            o = b0 * v + z1
            z1 = b1 * v - a1 * o + z2
            z2 = b2 * v - a2 * o
            y[i] = o
    return y


def shape(x, points):
    """Multiplies by a piecewise-linear envelope of (seconds, level) points."""
    out = list(x)
    j = 0
    for i in range(len(out)):
        t = i / SR
        while j + 1 < len(points) and t > points[j + 1][0]:
            j += 1
        if j + 1 >= len(points):
            g = points[-1][1]
        else:
            (t0, v0), (t1, v1) = points[j], points[j + 1]
            g = v0 + (v1 - v0) * (t - t0) / max(1e-9, t1 - t0) if t >= t0 else v0
        out[i] *= g
    return out


def decay(x, tau):
    return [v * math.exp(-i / SR / tau) for i, v in enumerate(x)]


def mix_into(dest, src, at, gain=1.0):
    start = n_of(at)
    for i, v in enumerate(src):
        k = start + i
        if 0 <= k < len(dest):
            dest[k] += v * gain


def silence(seconds):
    return [0.0] * n_of(seconds)


def sine(freq, n, phase=0.0):
    """`freq` is Hz or a function of the sample index."""
    out = []
    ph = phase
    for i in range(n):
        f = freq(i) if callable(freq) else freq
        ph += f / SR
        out.append(math.sin(TAU * ph))
    return out


def saw(freq, n):
    out = []
    ph = 0.0
    for i in range(n):
        f = freq(i) if callable(freq) else freq
        ph += f / SR
        ph -= int(ph)
        out.append(2 * ph - 1)
    return out


def pulse(freq, n, duty=0.25):
    return [1.0 if (v + 1) / 2 < duty else -1.0 for v in saw(freq, n)]


def voice(n, f0, vowel, breath=0.04, vib=5.2):
    """A sung vowel: a glottal saw through three moving formant filters."""
    src = []
    ph = 0.0
    for i in range(n):
        f = f0(i) * (1 + 0.012 * math.sin(TAU * vib * i / SR) + rng.uniform(-0.008, 0.008))
        ph += f / SR
        ph -= int(ph)
        src.append((2 * ph - 1) * 0.8 + rng.uniform(-1, 1) * breath)
    out = [0.0] * n
    for k, (w, q) in enumerate(((1.0, 6.0), (0.7, 8.0), (0.4, 10.0))):
        band = filt(src, "bp", lambda s, k=k: vowel(s)[k], q)
        for i in range(n):
            out[i] += w * band[i]
    return out


def crowd(make, count, spread):
    """Several slightly different copies, a little apart in time."""
    parts = [make(k) for k in range(count)]
    n = max(len(p) for p in parts) + n_of(spread)
    out = [0.0] * n
    for p in parts:
        mix_into(out, p, rng.uniform(0, spread), 1.0)
    return out


def save(name, x, peak=0.85):
    top = max(1e-9, max(abs(v) for v in x))
    fade = min(len(x), n_of(0.006))
    path = os.path.join(OUT, name + ".wav")
    os.makedirs(OUT, exist_ok=True)
    with wave.open(path, "wb") as w:
        w.setnchannels(1)
        w.setsampwidth(2)
        w.setframerate(SR)
        frames = bytearray()
        for i, v in enumerate(x):
            g = peak / top
            if i >= len(x) - fade:
                g *= (len(x) - i) / fade
            s = int(max(-1.0, min(1.0, v * g)) * 32767)
            frames += s.to_bytes(2, "little", signed=True)
        w.writeframes(bytes(frames))
    print("%-10s %.2fs %d KB" % (name, len(x) / SR, os.path.getsize(path) // 1024))


# ---- sounds ----

def s_tick():
    n = n_of(0.05)
    x = [math.sin(TAU * 1900 * i / SR) * math.exp(-i / SR * 130) + rng.uniform(-1, 1) * math.exp(-i / SR * 500) * 0.5 for i in range(n)]
    return x, 0.5


def s_ding():
    n = n_of(1.4)
    x = [0.0] * n
    for k, (m, a) in enumerate(((1.0, 1.0), (2.76, 0.5), (5.4, 0.25), (8.9, 0.12))):
        part = sine(880 * m, n)
        for i in range(n):
            x[i] += a * part[i] * math.exp(-i / SR * (2.5 + 2.0 * k))
    return x, 0.7


def s_tada():
    out = [0.0] * n_of(1.8)
    for at, m, d in ((0.0, 60, 0.16), (0.17, 64, 0.16), (0.34, 67, 0.16), (0.51, 72, 1.1)):
        n = n_of(d)
        note = saw(lambda i, m=m: hz(m) * (1 + 0.004 * math.sin(i / SR * 40)), n)
        note = filt(note, "lp", 2600, 0.8)
        note = shape(note, [(0, 0), (0.01, 1), (d * 0.7, 0.8), (d, 0)])
        mix_into(out, note, at)
    return out, 0.7


def s_clap():
    dur = 2.8
    out = [0.0] * n_of(dur)
    bed = filt(noise(len(out)), "bp", 2200, 0.7)
    out = [b * 0.12 for b in bed]
    out = shape(out, [(0, 0), (0.3, 1), (dur - 0.9, 1), (dur, 0)])
    for _ in range(170):
        t = rng.uniform(0.05, dur - 0.5)
        # Denser in the middle.
        if rng.random() > math.sin(math.pi * t / dur) ** 0.7:
            continue
        burst = filt(noise(n_of(0.03)), "bp", rng.uniform(1400, 3400), rng.uniform(0.8, 2.0))
        mix_into(out, decay(burst, 0.008), t, rng.uniform(0.4, 1.0))
    return out, 0.75


def s_cheer():
    n = n_of(1.9)

    def make(k):
        base = rng.uniform(200, 300)
        f0 = lambda i: base * (1 + 0.5 * min(1, i / SR / 0.9))
        # "yay": from "ah" towards "ay"
        vowel = lambda i: (700 - 330 * min(1, i / SR / 1.0), 1200 + 1000 * min(1, i / SR / 1.0), 2600)
        v = voice(n, f0, vowel, breath=0.08)
        return shape(v, [(0, 0), (0.12, 1), (1.3, 0.9), (1.9, 0)])

    out = crowd(make, 6, 0.25)
    claps = s_clap()[0][: n_of(1.6)]
    mix_into(out, claps, 0.0, 0.25 * max(abs(v) for v in out) / max(1e-9, max(abs(v) for v in claps)))
    return out, 0.8


def s_boo():
    n = n_of(2.2)

    def make(k):
        base = rng.uniform(95, 140)
        f0 = lambda i: base * (1 - 0.25 * min(1, i / SR / 1.8))
        vowel = lambda i: (300, 800 - 150 * min(1, i / SR / 1.8), 2400)
        v = voice(n, f0, vowel, breath=0.05, vib=4.2)
        return shape(v, [(0, 0), (0.35, 1), (1.6, 0.9), (2.2, 0)])

    return crowd(make, 7, 0.35), 0.8


def s_aww():
    n = n_of(2.0)

    def make(k):
        base = rng.uniform(230, 330)
        f0 = lambda i: base * (1 - 0.22 * min(1, max(0, i / SR - 0.3) / 1.4))
        vowel = lambda i: (760, 1150 - 100 * min(1, i / SR / 1.2), 2700)
        v = voice(n, f0, vowel, breath=0.05, vib=5.6)
        return shape(v, [(0, 0), (0.2, 1), (1.3, 0.8), (2.0, 0)])

    return crowd(make, 6, 0.3), 0.75


def s_gasp():
    out = [0.0] * n_of(1.1)
    for k in range(4):
        n = n_of(0.5)
        breath = filt(noise(n), "bp", lambda s: 700 + 1800 * s / n, 1.2)
        breath = shape(breath, [(0, 0), (0.35, 1), (0.5, 0)])
        mix_into(out, breath, rng.uniform(0, 0.12))
    return out, 0.7


def s_laugh():
    out = [0.0] * n_of(2.3)
    for v in range(4):
        base = rng.uniform(180, 300)
        start = rng.uniform(0, 0.15)
        for k in range(6):
            n = n_of(0.13)
            f0 = lambda i, b=base, k=k: b * (1 - 0.05 * k)
            vowel = lambda i: (780, 1250, 2600)
            ha = voice(n, f0, vowel, breath=0.18)
            ha = shape(ha, [(0, 0), (0.015, 1), (0.1, 0.5), (0.13, 0)])
            mix_into(out, ha, start + k * 0.2 + rng.uniform(-0.01, 0.01), 1.0 - 0.1 * k)
    return out, 0.75


def s_fart():
    dur = 1.15
    n = n_of(dur)
    pulses = [0.0] * n
    t = 0.0
    while t < dur - 0.1:
        f = 120 - 70 * (t / dur) + rng.uniform(-14, 14)
        pulses[int(t * SR)] = 1.0
        t += 1.0 / max(35.0, f)
    body = filt(pulses, "lp", 380, 1.4)
    body = filt(body, "bp", 190, 0.9)
    rasp = filt(noise(n), "lp", 330, 0.8)
    am = [0.55 + 0.45 * math.sin(TAU * 31 * i / SR) if i / SR > 0.75 else 1.0 for i in range(n)]
    x = [(body[i] * 4 + rasp[i] * 0.7) * am[i] for i in range(n)]
    x = shape(x, [(0, 0), (0.04, 1), (0.6, 0.95), (dur, 0)])
    return x, 0.85


def s_wah():
    out = [0.0] * n_of(3.4)
    at = 0.0
    for midi, d, last in ((58, 0.58, False), (57, 0.58, False), (56, 0.58, False), (55, 1.5, True)):
        n = n_of(d)
        vib = 0.018 if last else 0.0
        note = saw(lambda i, m=midi: hz(m) * (1 + vib * math.sin(TAU * 5.5 * i / SR) * min(1, i / SR / 0.5)), n)
        note = [a + 0.6 * b for a, b in zip(note, saw(lambda i, m=midi: hz(m) * 1.006, n))]
        cut = lambda s, n=n: 380 + 1500 * math.sin(math.pi * min(1.0, (s / n) * (1.0 if not last else 0.55))) ** 1.5
        note = filt(note, "lp", cut, 2.2)
        note = shape(note, [(0, 0), (0.02, 1), (d - 0.08, 0.8), (d, 0)])
        mix_into(out, note, at)
        at += d + 0.04
    return out, 0.8


def s_slide():
    n = n_of(1.3)
    x = sine(lambda i: 420 + 1900 * (min(i / SR, 0.95) / 0.95) ** 1.5 * (1 + 0.012 * math.sin(TAU * 6.5 * i / SR)), n)
    x = shape(x, [(0, 0), (0.03, 1), (0.95, 1), (1.3, 0)])
    return x, 0.7


def s_slide_down():
    n = n_of(1.0)
    x = sine(lambda i: 2000 - 1600 * (min(i / SR, 0.9) / 0.9) ** 1.2 + 25 * math.sin(TAU * 7 * i / SR), n)
    x = shape(x, [(0, 0), (0.03, 1), (0.8, 0.9), (1.0, 0)])
    return x, 0.7


def s_kazoo():
    out = [0.0] * n_of(2.0)
    at = 0.0
    for midi, d in ((72, 0.2), (72, 0.2), (72, 0.2), (76, 0.2), (79, 0.2), (76, 0.15), (79, 0.7)):
        n = n_of(d)
        note = pulse(lambda i, m=midi: hz(m) * (1 + 0.006 * math.sin(TAU * 6 * i / SR)), n, 0.3)
        buzz = [1 + 0.7 * math.sin(TAU * 190 * i / SR) for i in range(n)]
        note = filt([a * b for a, b in zip(note, buzz)], "bp", 1250, 1.6)
        note = shape(note, [(0, 0), (0.015, 1), (d - 0.04, 0.9), (d, 0)])
        mix_into(out, note, at)
        at += d + 0.015
    return out, 0.75


def s_pop():
    n = n_of(0.8)
    bang = filt(noise(n), "lp", 3200, 0.7)
    bang = decay(bang, 0.03)
    thump = decay(sine(lambda i: 60 + 190 * math.exp(-i / SR * 40), n), 0.07)
    hiss = shape(filt(noise(n), "hp", 3500, 0.7), [(0, 0), (0.05, 0.35), (0.6, 0.12), (0.8, 0)])
    return [a * 1.0 + b * 0.9 + c for a, b, c in zip(bang, thump, hiss)], 0.9


def s_heartbeat():
    out = [0.0] * n_of(0.86)
    for at, f, a in ((0.0, 62, 1.0), (0.3, 54, 0.75)):
        n = n_of(0.22)
        thump = decay(sine(lambda i: f * (1.0 - 0.3 * min(1, i / SR / 0.1)), n), 0.06)
        click = decay(filt(noise(n), "lp", 160, 0.7), 0.03)
        mix_into(out, [t * a + c * 0.8 * a for t, c in zip(thump, click)], at)
    return out, 0.9


def s_thunder():
    n = n_of(3.8)
    low = filt(filt(noise(n), "lp", 160, 0.9), "lp", 220, 0.9)
    low = shape(low, [(0, 0), (0.25, 1), (0.9, 0.9), (3.8, 0)])
    wobble = [0.7 + 0.3 * math.sin(TAU * (2.2 + 1.4 * math.sin(i / SR * 1.3)) * i / SR) for i in range(n)]
    low = [a * b for a, b in zip(low, wobble)]
    crack = [0.0] * n
    for _ in range(26):
        t = rng.uniform(0.0, 0.5)
        mix_into(crack, decay(filt(noise(n_of(0.05)), "hp", 1200, 0.8), 0.012), t, rng.uniform(0.3, 1))
    return [a * 3 + b * 0.5 for a, b in zip(low, crack)], 0.9


def s_rain():
    n = n_of(4.0)
    bed = filt(filt(noise(n), "hp", 1800, 0.7), "lp", 8000, 0.7)
    out = [v * 0.5 for v in bed]
    for _ in range(230):
        d = filt(noise(n_of(0.012)), "bp", rng.uniform(2000, 6000), 3.0)
        mix_into(out, decay(d, 0.004), rng.uniform(0, 3.95), rng.uniform(0.3, 1.0))
    return out, 0.5


def s_splash():
    n = n_of(1.4)
    out = decay(filt(noise(n), "bp", 1800, 0.6), 0.14)
    out = [v * 1.2 for v in out]
    for _ in range(9):
        k = n_of(0.09)
        f0 = rng.uniform(300, 700)
        bub = decay(sine(lambda i, f0=f0: f0 * (1 + 2.5 * i / k), k), 0.03)
        mix_into(out, bub, rng.uniform(0.05, 0.9), rng.uniform(0.15, 0.4))
    return out, 0.85


def s_rocket():
    n = n_of(3.8)
    roar = filt(noise(n), "lp", lambda s: 260 + 1800 * min(1.0, s / SR / 2.2), 0.8)
    roar = shape(roar, [(0, 0), (0.5, 0.8), (2.5, 1), (3.8, 0)])
    rumble = shape(saw(58, n), [(0, 0), (0.4, 0.7), (3.0, 0.7), (3.8, 0)])
    rumble = filt(rumble, "lp", 220, 0.8)
    crackle = [0.0] * n
    for _ in range(140):
        mix_into(crackle, decay(filt(noise(n_of(0.02)), "hp", 900, 0.8), 0.006), rng.uniform(0.2, 3.5), rng.uniform(0.2, 0.8))
    return [a * 2.0 + b * 0.8 + c * 0.35 for a, b, c in zip(roar, rumble, crackle)], 0.85


def s_rumble():
    n = n_of(1.8)
    low = filt(noise(n), "lp", 95, 1.0)
    trem = [0.6 + 0.4 * math.sin(TAU * 9 * i / SR) for i in range(n)]
    hum = sine(38, n)
    x = [(low[i] * 4 + hum[i] * 0.5) * trem[i] for i in range(n)]
    return shape(x, [(0, 0), (0.3, 1), (1.5, 1), (1.8, 0)]), 0.8


def s_splat():
    n = n_of(0.45)
    thump = decay(sine(lambda i: 55 + 110 * math.exp(-i / SR * 30), n), 0.06)
    wet = decay(filt(noise(n), "bp", 900, 0.9), 0.05)
    squelch = decay(sine(lambda i: 350 + 280 * math.sin(TAU * 14 * i / SR), n), 0.05)
    return [a + b * 1.2 + c * 0.25 for a, b, c in zip(thump, wet, squelch)], 0.85


def s_crash():
    n = n_of(2.2)
    out = decay(filt(noise(n), "bp", 3000, 0.5), 0.22)
    out = [v * 1.2 for v in out]
    for f, tau in ((1230, 0.5), (1670, 0.4), (2310, 0.3), (3050, 0.25)):
        part = decay(sine(f, n), tau)
        out = [a + 0.18 * b for a, b in zip(out, part)]
    for at, a in ((0.45, 0.6), (0.72, 0.4), (0.92, 0.25)):
        thud = decay(sine(lambda i: 90 + 60 * math.exp(-i / SR * 25), n_of(0.25)), 0.05)
        clink = decay(filt(noise(n_of(0.25)), "bp", 2400, 1.4), 0.04)
        mix_into(out, [t + 0.6 * c for t, c in zip(thud, clink)], at, a)
    return out, 0.9


def s_skid():
    n = n_of(0.85)
    squeal = sine(lambda i: 2300 - 1300 * (i / n) ** 0.8 + 60 * math.sin(TAU * 21 * i / SR), n)
    harm = sine(lambda i: 4600 - 2600 * (i / n) ** 0.8, n)
    rubber = filt(noise(n), "bp", 1500, 1.2)
    x = [a * 0.8 + b * 0.25 + c * 0.45 for a, b, c in zip(squeal, harm, rubber)]
    return shape(x, [(0, 0), (0.04, 1), (0.6, 0.8), (0.85, 0)]), 0.75


def s_scamper():
    out = [0.0] * n_of(1.3)
    t = 0.0
    k = 0
    while t < 1.15:
        f = 480 if k % 2 == 0 else 620
        n = n_of(0.07)
        blip = decay(sine(lambda i, f=f: f * (1 + 0.3 * math.exp(-i / SR * 60)), n), 0.018)
        mix_into(out, blip, t, 0.9)
        t += 0.058
        k += 1
    puff = shape(filt(noise(len(out)), "bp", 1200, 0.6), [(0, 0.1), (1.1, 0.25), (1.3, 0)])
    return [a + 0.15 * b for a, b in zip(out, puff)], 0.7


def s_whoosh():
    n = n_of(0.65)
    x = filt(noise(n), "bp", lambda s: 250 + 3200 * math.sin(math.pi * s / n) ** 1.5, 0.9)
    return shape(x, [(0, 0), (0.3, 1), (0.65, 0)]), 0.7


def s_boing():
    n = n_of(0.7)
    f = lambda i: 190 + 520 * math.sin(math.pi * min(1.0, i / SR / 0.14)) ** 0.7 * math.exp(-i / SR * 2.0) + 40 * math.sin(TAU * 17 * i / SR) * math.exp(-i / SR * 5)
    return decay(sine(f, n), 0.2), 0.7


def s_thud():
    n = n_of(0.22)
    thump = decay(sine(lambda i: 70 + 90 * math.exp(-i / SR * 40), n), 0.05)
    click = decay(filt(noise(n), "lp", 500, 0.7), 0.015)
    return [a + 0.5 * b for a, b in zip(thump, click)], 0.7


def s_step():
    n = n_of(0.1)
    pat = decay(sine(lambda i: 260 - 120 * min(1, i / n), n), 0.02)
    tap = decay(filt(noise(n), "bp", 900, 1.2), 0.008)
    return [a + 0.6 * b for a, b in zip(pat, tap)], 0.5


def s_zap():
    n = n_of(0.7)
    buzz = pulse(lambda i: 55 + 3000 * math.exp(-i / SR * 9), n, 0.35)
    crack = filt(noise(n), "hp", 1500, 0.8)
    x = [a * 0.7 + b * 0.7 for a, b in zip(buzz, crack)]
    return decay(x, 0.14), 0.85


def s_freeze():
    n = n_of(1.5)
    out = [0.0] * n
    for _ in range(26):
        f = rng.uniform(1800, 5200)
        k = n_of(0.35)
        mix_into(out, decay(sine(f, k), 0.07), rng.uniform(0, 1.1), rng.uniform(0.2, 0.6))
    creak = filt(noise(n), "bp", lambda s: 500 + 400 * math.sin(s / SR * 18), 5.0)
    creak = shape(creak, [(0, 0), (0.2, 0.3), (1.0, 0.25), (1.5, 0)])
    return [a + 0.4 * b for a, b in zip(out, creak)], 0.7


def s_sparkle():
    out = [0.0] * n_of(0.9)
    for k, m in enumerate((84, 88, 91, 96, 100)):
        n = n_of(0.35)
        mix_into(out, decay(sine(hz(m), n), 0.09), 0.06 * k, 0.8 - 0.1 * k)
    return out, 0.55


def s_drumroll():
    dur = 4.6
    out = [0.0] * n_of(dur + 0.2)
    t = 0.0
    while t < dur:
        hit = decay(filt(noise(n_of(0.05)), "bp", 1900, 0.9), 0.012)
        lvl = 0.15 + 0.85 * (t / dur) ** 1.3
        mix_into(out, hit, t, lvl * rng.uniform(0.8, 1.0))
        t += 0.032 - 0.012 * (t / dur)
    return out, 0.6


def s_disco():
    beat = 0.5
    out = [0.0] * n_of(beat * 8)
    for b in range(8):
        t = b * beat
        kick = decay(sine(lambda i: 45 + 110 * math.exp(-i / SR * 28), n_of(0.3)), 0.11)
        mix_into(out, kick, t, 1.0)
        hat = decay(filt(noise(n_of(0.07)), "hp", 5000, 0.7), 0.018)
        mix_into(out, hat, t + beat / 2, 0.35)
        if b % 2 == 1:
            clap = decay(filt(noise(n_of(0.18)), "bp", 1600, 0.8), 0.04)
            mix_into(out, clap, t, 0.6)
    bass = (33, 33, 40, 33, 36, 33, 40, 38)
    for k in range(16):
        n = n_of(0.22)
        note = filt(saw(hz(bass[k % 8] + 12), n), "lp", 420, 1.2)
        mix_into(out, decay(note, 0.12), k * beat / 2, 0.8)
    for k in range(16):
        if k % 4 == 3:
            chord = [0.0] * n_of(0.18)
            for m in (57, 60, 64):
                part = saw(hz(m), len(chord))
                chord = [a + b * 0.33 for a, b in zip(chord, part)]
            mix_into(out, decay(filt(chord, "lp", 2200, 0.8), 0.07), k * beat / 2, 0.35)
    return out, 0.7


def s_whistle():
    """A firework rocket climbing."""
    n = n_of(0.8)
    x = sine(lambda i: 700 + 1900 * (i / n) ** 1.3 + 35 * math.sin(TAU * 38 * i / SR), n)
    hiss = filt(noise(n), "hp", 3000, 0.7)
    x = [a + 0.18 * b for a, b in zip(x, hiss)]
    return shape(x, [(0, 0), (0.05, 0.7), (0.65, 1), (0.8, 0)]), 0.6


def s_boom():
    """A firework bursting: a thump, a crack and a rolling echo."""
    n = n_of(1.8)
    thump = decay(sine(lambda i: 50 + 130 * math.exp(-i / SR * 22), n), 0.18)
    crack = decay(filt(noise(n), "hp", 800, 0.7), 0.05)
    rumble = decay(filt(noise(n), "lp", 260, 0.8), 0.35)
    return [a * 1.0 + b * 0.8 + c * 1.6 for a, b, c in zip(thump, crack, rumble)], 0.95


def s_crackle():
    n = n_of(1.3)
    out = [0.0] * n
    for _ in range(110):
        t = rng.uniform(0, 1.1) ** 1.4
        click = decay(filt(noise(n_of(0.02)), "hp", rng.uniform(1500, 4500), 1.0), 0.004)
        mix_into(out, click, t, rng.uniform(0.2, 1.0))
    return out, 0.6


def s_explosion():
    n = n_of(2.6)
    blast = decay(filt(noise(n), "lp", lambda s: 2600 * math.exp(-s / SR * 2.2) + 120, 0.8), 0.45)
    thump = decay(sine(lambda i: 38 + 110 * math.exp(-i / SR * 14), n), 0.4)
    crack = decay(filt(noise(n), "hp", 900, 0.7), 0.06)
    out = [a * 1.4 + b * 1.4 + c * 0.9 for a, b, c in zip(blast, thump, crack)]
    for _ in range(40):
        clatter = decay(filt(noise(n_of(0.06)), "bp", rng.uniform(1200, 4000), 2.0), 0.015)
        mix_into(out, clatter, rng.uniform(0.2, 1.9), rng.uniform(0.1, 0.4))
    return out, 0.98


def s_fuse():
    n = n_of(1.9)
    hiss = filt(noise(n), "hp", 2500, 0.8)
    hiss = shape(hiss, [(0, 0.2), (1.8, 1), (1.9, 0)])
    out = [v * 0.5 for v in hiss]
    t = 0.0
    while t < 1.8:
        tick = decay(sine(1400, n_of(0.05)), 0.012)
        mix_into(out, tick, t, 0.5 + 0.5 * t / 1.8)
        t += 0.35 - 0.22 * t / 1.8
    return out, 0.6


SOUNDS = {
    "tick": s_tick, "ding": s_ding, "tada": s_tada, "drumroll": s_drumroll,
    "clap": s_clap, "cheer": s_cheer, "boo": s_boo, "aww": s_aww, "gasp": s_gasp, "laugh": s_laugh,
    "fart": s_fart, "wah": s_wah, "slide": s_slide, "slide_down": s_slide_down, "kazoo": s_kazoo, "pop": s_pop,
    "heartbeat": s_heartbeat, "thunder": s_thunder, "rain": s_rain, "splash": s_splash,
    "rocket": s_rocket, "rumble": s_rumble, "splat": s_splat, "crash": s_crash, "skid": s_skid,
    "scamper": s_scamper, "whoosh": s_whoosh, "boing": s_boing, "thud": s_thud, "step": s_step,
    "zap": s_zap, "freeze": s_freeze, "sparkle": s_sparkle, "disco": s_disco,
    "whistle": s_whistle, "boom": s_boom, "crackle": s_crackle, "explosion": s_explosion, "fuse": s_fuse,
}

if __name__ == "__main__":
    wanted = sys.argv[1:] or list(SOUNDS)
    for name in wanted:
        x, peak = SOUNDS[name]()
        save(name, x, peak)
