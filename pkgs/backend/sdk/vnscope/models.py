from dataclasses import dataclass


@dataclass
class CandleStick:
    t: int
    o: float
    h: float
    l: float
    c: float
    v: int
