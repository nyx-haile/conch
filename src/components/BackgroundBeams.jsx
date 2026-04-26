import { motion } from "motion/react";

const beamPaths = [
  "M-80 220 C 170 80, 370 380, 720 170 S 1050 20, 1280 180",
  "M-120 520 C 160 310, 330 650, 650 430 S 980 280, 1320 470",
  "M60 760 C 250 520, 520 810, 780 610 S 1090 500, 1380 690",
];

export function BackgroundBeams() {
  return (
    <div className="beams" aria-hidden="true">
      <motion.div
        className="aurora aurora-one"
        animate={{ opacity: [0.5, 0.9, 0.55], scale: [1, 1.08, 1] }}
        transition={{ duration: 9, repeat: Infinity, ease: "easeInOut" }}
      />
      <motion.div
        className="aurora aurora-two"
        animate={{ opacity: [0.35, 0.7, 0.35], scale: [1.04, 0.95, 1.04] }}
        transition={{ duration: 11, repeat: Infinity, ease: "easeInOut" }}
      />
      <svg viewBox="0 0 1280 820" preserveAspectRatio="none" role="presentation">
        <defs>
          <linearGradient id="beam-gradient" x1="0%" y1="0%" x2="100%" y2="0%">
            <stop offset="0%" stopColor="#22d3ee" stopOpacity="0" />
            <stop offset="35%" stopColor="#22d3ee" stopOpacity="0.82" />
            <stop offset="65%" stopColor="#ff4fd8" stopOpacity="0.74" />
            <stop offset="100%" stopColor="#fef08a" stopOpacity="0" />
          </linearGradient>
        </defs>
        {beamPaths.map((path, index) => (
          <motion.path
            key={path}
            d={path}
            fill="none"
            stroke="url(#beam-gradient)"
            strokeWidth={index === 1 ? 1.4 : 1}
            strokeLinecap="round"
            initial={{ pathLength: 0, opacity: 0 }}
            animate={{ pathLength: [0, 1, 1], opacity: [0, 0.72, 0] }}
            transition={{
              duration: 7 + index * 1.2,
              repeat: Infinity,
              ease: "easeInOut",
              delay: index * 1.5,
            }}
          />
        ))}
      </svg>
    </div>
  );
}
