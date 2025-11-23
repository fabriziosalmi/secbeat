import { useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';

interface Packet {
  id: number;
  y: number;
  isAttack: boolean;
  speed: number;
}

const COLORS = {
  cyber: '#00f0ff',
  deep: '#050505',
  void: '#0a0a0a',
  danger: '#ff3366',
  neonGreen: '#39ff14',
  rust: '#dea584',
};

export default function TrafficHud() {
  const [packets, setPackets] = useState<Packet[]>([]);
  const [stats, setStats] = useState({
    pps: 0,
    dropped: 0,
    passed: 0,
  });
  const [shieldActive, setShieldActive] = useState(false);

  useEffect(() => {
    // Generate packets continuously
    const packetInterval = setInterval(() => {
      const newPacket: Packet = {
        id: Math.random(),
        y: Math.random() * 80 + 10, // Random Y position (10-90%)
        isAttack: Math.random() > 0.6, // 40% chance of being an attack
        speed: 2 + Math.random() * 2, // Random speed
      };

      setPackets((prev) => [...prev, newPacket]);

      // Simulate stats
      setStats((prev) => ({
        pps: Math.floor(2000000 + Math.random() * 500000),
        dropped: prev.dropped + (newPacket.isAttack ? 1 : 0),
        passed: prev.passed + (newPacket.isAttack ? 0 : 1),
      }));
    }, 150);

    // Remove old packets
    const cleanupInterval = setInterval(() => {
      setPackets((prev) => prev.filter((p) => true)); // Will be removed by animation exit
    }, 100);

    return () => {
      clearInterval(packetInterval);
      clearInterval(cleanupInterval);
    };
  }, []);

  const handlePacketComplete = (packet: Packet) => {
    setPackets((prev) => prev.filter((p) => p.id !== packet.id));
    if (packet.isAttack) {
      setShieldActive(true);
      setTimeout(() => setShieldActive(false), 100);
    }
  };

  return (
    <div
      className="relative w-full h-[400px] overflow-hidden rounded-lg border"
      style={{
        borderColor: COLORS.cyber,
        background: `linear-gradient(to bottom right, ${COLORS.deep}, ${COLORS.void})`
      }}
    >
      {/* Background grid */}
      <div className="absolute inset-0 opacity-20">
        <div className="grid grid-cols-12 h-full">
          {Array.from({ length: 12 }).map((_, i) => (
            <div key={i} className="border-r" style={{ borderColor: COLORS.cyber }} />
          ))}
        </div>
      </div>

      {/* Central Shield (SecBeat) */}
      <div className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 z-10">
        <motion.div
          animate={{
            scale: shieldActive ? 1.2 : 1,
            filter: shieldActive
              ? 'drop-shadow(0 0 20px rgba(255, 51, 102, 0.8))'
              : 'drop-shadow(0 0 10px rgba(0, 240, 255, 0.5))',
          }}
          transition={{ duration: 0.1 }}
        >
          <svg width="80" height="80" viewBox="0 0 128 128">
            <path
              d="M64 10L20 30V60C20 85 35 105 64 118C93 105 108 85 108 60V30L64 10Z"
              stroke={shieldActive ? COLORS.danger : COLORS.cyber}
              strokeWidth="3"
              fill="rgba(5, 5, 5, 0.8)"
            />
            <text
              x="64"
              y="70"
              textAnchor="middle"
              fill={shieldActive ? COLORS.danger : COLORS.cyber}
              fontSize="20"
              fontWeight="bold"
              fontFamily="'JetBrains Mono', monospace"
            >
              XDP
            </text>
          </svg>
        </motion.div>
      </div>

      {/* Packets */}
      <AnimatePresence>
        {packets.map((packet) => (
          <motion.div
            key={packet.id}
            initial={{ x: -20, opacity: 0 }}
            animate={{
              x: packet.isAttack ? '50%' : '110%',
              opacity: packet.isAttack ? [1, 1, 0] : 1,
            }}
            exit={{ opacity: 0 }}
            transition={{
              duration: packet.speed,
              ease: 'linear',
            }}
            onAnimationComplete={() => handlePacketComplete(packet)}
            style={{
              position: 'absolute',
              top: `${packet.y}%`,
              left: 0,
            }}
            className="flex items-center gap-1"
          >
            <div
              className="w-3 h-3 rounded-full"
              style={{
                backgroundColor: packet.isAttack ? COLORS.danger : COLORS.neonGreen,
                boxShadow: packet.isAttack
                  ? `0 0 10px ${COLORS.danger}`
                  : `0 0 5px ${COLORS.neonGreen}`,
              }}
            />
            <div
              className="h-px w-8"
              style={{
                backgroundColor: packet.isAttack ? COLORS.danger : COLORS.neonGreen,
                opacity: 0.3,
              }}
            />
          </motion.div>
        ))}
      </AnimatePresence>

      {/* Stats Overlay */}
      <div className="absolute bottom-4 left-4 right-4 flex justify-between items-end">
        <div className="space-y-1">
          <div className="text-xs font-mono opacity-70" style={{ color: COLORS.cyber }}>
            PACKETS/SEC
          </div>
          <div className="text-2xl font-bold font-mono" style={{ color: COLORS.cyber }}>
            {stats.pps.toLocaleString()}
          </div>
        </div>
        <div className="space-y-1">
          <div className="text-xs font-mono opacity-70" style={{ color: COLORS.danger }}>
            DROPPED
          </div>
          <div className="text-2xl font-bold font-mono" style={{ color: COLORS.danger }}>
            {stats.dropped.toLocaleString()}
          </div>
        </div>
        <div className="space-y-1">
          <div className="text-xs font-mono opacity-70" style={{ color: COLORS.neonGreen }}>
            PASSED
          </div>
          <div className="text-2xl font-bold font-mono" style={{ color: COLORS.neonGreen }}>
            {stats.passed.toLocaleString()}
          </div>
        </div>
      </div>

      {/* Latency indicator */}
      <div className="absolute top-4 right-4">
        <div className="text-xs font-mono opacity-70" style={{ color: COLORS.rust }}>LATENCY</div>
        <div className="text-lg font-bold font-mono" style={{ color: COLORS.rust }}>
          {Math.floor(10 + Math.random() * 5)}µs
        </div>
      </div>
    </div>
  );
}
