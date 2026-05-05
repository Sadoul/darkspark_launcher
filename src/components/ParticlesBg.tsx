import { useEffect, useState } from "react";
import Particles, { initParticlesEngine } from "@tsparticles/react";
import { loadSlim } from "@tsparticles/slim";

export default function ParticlesBg() {
  const [ready, setReady] = useState(false);

  useEffect(() => {
    initParticlesEngine(async (engine) => {
      await loadSlim(engine);
    }).then(() => setReady(true));
  }, []);

  if (!ready) {
    return null;
  }

  return (
    <Particles
      className="particles-bg"
      options={{
        fullScreen: false,
        fpsLimit: 60,
        particles: {
          number: {
            value: 50,
            density: {
              enable: true,
            },
          },
          color: {
            value: ["#D4793A", "#E8A87C", "#C4A882", "#B8612A"],
          },
          shape: {
            type: "circle",
          },
          opacity: {
            value: { min: 0.1, max: 0.4 },
            animation: {
              enable: true,
              speed: 0.5,
              sync: false,
            },
          },
          size: {
            value: { min: 1, max: 3 },
            animation: {
              enable: true,
              speed: 1,
              sync: false,
            },
          },
          links: {
            enable: true,
            distance: 150,
            color: "#D4793A",
            opacity: 0.08,
            width: 1,
          },
          move: {
            enable: true,
            speed: 0.6,
            direction: "none",
            random: true,
            straight: false,
            outModes: {
              default: "bounce",
            },
          },
        },
        interactivity: {
          events: {
            onHover: {
              enable: true,
              mode: "grab",
            },
          },
          modes: {
            grab: {
              distance: 140,
              links: {
                opacity: 0.25,
              },
            },
          },
        },
        detectRetina: true,
      }}
    />
  );
}