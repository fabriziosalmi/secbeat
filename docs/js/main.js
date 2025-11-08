// ===================================
// SecBeat - Interactive Features
// ===================================

// Intersection Observer for scroll animations
const observerOptions = {
    threshold: 0.1,
    rootMargin: '0px 0px -50px 0px'
};

const fadeInObserver = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
        if (entry.isIntersecting) {
            entry.target.style.opacity = '1';
            entry.target.style.transform = 'translateY(0)';
        }
    });
}, observerOptions);

// Initialize scroll animations
document.addEventListener('DOMContentLoaded', () => {
    // Add fade-in animation to sections
    const sections = document.querySelectorAll('section');
    sections.forEach(section => {
        section.style.opacity = '0';
        section.style.transform = 'translateY(30px)';
        section.style.transition = 'opacity 0.8s ease-out, transform 0.8s ease-out';
        fadeInObserver.observe(section);
    });

    // Initialize all features
    initTerminalAnimation();
    initStatCounters();
    initSmoothScroll();
    initCardHoverEffects();
    initParallaxEffect();
    initNodeResilience();
});

// ===================================
// Terminal Animation
// ===================================

function initTerminalAnimation() {
    const commands = [
        { 
            prompt: '$ ', 
            command: 'cargo run --release',
            output: [
                '   Compiling secbeat v1.0.0',
                '    Finished release [optimized] target(s) in 12.34s',
                '     Running `target/release/mitigation-node`'
            ],
            delay: 100
        },
        {
            prompt: '> ',
            command: 'Starting SecBeat Mitigation Node...',
            output: [
                '✓ SYN Proxy initialized',
                '✓ WAF engine loaded with 50,000 rules',
                '✓ Connected to orchestrator at nats://orchestrator:4222',
                '✓ Prometheus metrics endpoint: :9090/metrics'
            ],
            delay: 80
        },
        {
            prompt: '📊 ',
            command: 'Real-time metrics:',
            output: [
                'Packets/sec: 2.5M',
                'Blocked attacks: 1,247',
                'CPU usage: 12%',
                'Memory: 256MB',
                'Uptime: 99.99%'
            ],
            delay: 60
        }
    ];

    let currentCommand = 0;
    let currentChar = 0;
    let currentOutput = 0;

    const terminalBody = document.querySelector('.terminal-body');
    if (!terminalBody) return;

    function typeCommand() {
        if (currentCommand >= commands.length) {
            // Restart animation
            setTimeout(() => {
                terminalBody.innerHTML = '';
                currentCommand = 0;
                currentChar = 0;
                currentOutput = 0;
                typeCommand();
            }, 3000);
            return;
        }

        const cmd = commands[currentCommand];
        
        if (currentChar === 0) {
            const line = document.createElement('div');
            line.className = 'terminal-line';
            line.innerHTML = `<span class="prompt">${cmd.prompt}</span><span class="command"></span><span class="cursor">▋</span>`;
            terminalBody.appendChild(line);
        }

        const commandSpan = terminalBody.lastElementChild.querySelector('.command');
        const cursor = terminalBody.lastElementChild.querySelector('.cursor');

        if (currentChar < cmd.command.length) {
            commandSpan.textContent += cmd.command[currentChar];
            currentChar++;
            setTimeout(typeCommand, cmd.delay);
        } else {
            cursor.remove();
            
            if (currentOutput < cmd.output.length) {
                const outputLine = document.createElement('div');
                outputLine.className = 'terminal-output';
                outputLine.innerHTML = formatOutput(cmd.output[currentOutput]);
                terminalBody.appendChild(outputLine);
                currentOutput++;
                setTimeout(typeCommand, 300);
            } else {
                currentCommand++;
                currentChar = 0;
                currentOutput = 0;
                setTimeout(typeCommand, 1000);
            }
        }

        // Auto-scroll
        terminalBody.scrollTop = terminalBody.scrollHeight;
    }

    function formatOutput(text) {
        if (text.startsWith('✓')) {
            return `<span class="success">${text}</span>`;
        } else if (text.includes(':')) {
            const [key, value] = text.split(':');
            return `<span class="metric">${key}:</span> <span class="value">${value}</span>`;
        }
        return text;
    }

    typeCommand();
}

// ===================================
// Stat Counters
// ===================================

function initStatCounters() {
    const stats = [
        { id: 'packets-stat', target: 2500000, suffix: '/sec', duration: 2000 },
        { id: 'attacks-stat', target: 99, suffix: '%', duration: 2000 },
        { id: 'latency-stat', target: 0.3, suffix: 'ms', decimals: 1, duration: 2000 },
        { id: 'uptime-stat', target: 99.99, suffix: '%', decimals: 2, duration: 2000 }
    ];

    const formatNumber = (num, decimals = 0) => {
        if (num >= 1000000) {
            return (num / 1000000).toFixed(decimals) + 'M';
        } else if (num >= 1000) {
            return (num / 1000).toFixed(decimals) + 'K';
        }
        return num.toFixed(decimals);
    };

    stats.forEach(stat => {
        const element = document.getElementById(stat.id);
        if (!element) return;

        const observer = new IntersectionObserver((entries) => {
            entries.forEach(entry => {
                if (entry.isIntersecting && !element.dataset.counted) {
                    element.dataset.counted = 'true';
                    animateCounter(element, stat);
                }
            });
        }, { threshold: 0.5 });

        observer.observe(element);
    });

    function animateCounter(element, stat) {
        const startTime = Date.now();
        const startValue = 0;
        
        const animate = () => {
            const elapsed = Date.now() - startTime;
            const progress = Math.min(elapsed / stat.duration, 1);
            
            // Easing function
            const easeOutQuart = 1 - Math.pow(1 - progress, 4);
            const currentValue = startValue + (stat.target - startValue) * easeOutQuart;
            
            element.textContent = formatNumber(currentValue, stat.decimals || 0) + (stat.suffix || '');
            
            if (progress < 1) {
                requestAnimationFrame(animate);
            } else {
                element.textContent = formatNumber(stat.target, stat.decimals || 0) + (stat.suffix || '');
            }
        };
        
        animate();
    }
}

// ===================================
// Smooth Scroll
// ===================================

function initSmoothScroll() {
    document.querySelectorAll('a[href^="#"]').forEach(anchor => {
        anchor.addEventListener('click', function (e) {
            e.preventDefault();
            const target = document.querySelector(this.getAttribute('href'));
            
            if (target) {
                const offsetTop = target.offsetTop - 80;
                window.scrollTo({
                    top: offsetTop,
                    behavior: 'smooth'
                });
            }
        });
    });
}

// ===================================
// Card Hover Effects
// ===================================

function initCardHoverEffects() {
    const cards = document.querySelectorAll('.feature-card, .deployment-card, .doc-card, .arch-component');
    
    cards.forEach(card => {
        card.addEventListener('mouseenter', function(e) {
            this.style.transform = 'translateY(-8px) scale(1.02)';
        });
        
        card.addEventListener('mouseleave', function(e) {
            this.style.transform = 'translateY(0) scale(1)';
        });

        // 3D tilt effect on mouse move
        card.addEventListener('mousemove', function(e) {
            const rect = this.getBoundingClientRect();
            const x = e.clientX - rect.left;
            const y = e.clientY - rect.top;
            
            const centerX = rect.width / 2;
            const centerY = rect.height / 2;
            
            const rotateX = (y - centerY) / 20;
            const rotateY = (centerX - x) / 20;
            
            this.style.transform = `translateY(-8px) rotateX(${rotateX}deg) rotateY(${rotateY}deg)`;
        });
        
        card.addEventListener('mouseleave', function() {
            this.style.transform = 'translateY(0) rotateX(0) rotateY(0)';
        });
    });
}

// ===================================
// Parallax Effect
// ===================================

function initParallaxEffect() {
    let ticking = false;
    
    window.addEventListener('scroll', () => {
        if (!ticking) {
            window.requestAnimationFrame(() => {
                updateParallax();
                ticking = false;
            });
            ticking = true;
        }
    });
    
    function updateParallax() {
        const scrolled = window.pageYOffset;
        
        // Move stars slower than scroll
        const stars = document.querySelector('.stars');
        if (stars) {
            stars.style.transform = `translateY(${scrolled * 0.5}px)`;
        }
        
        // Move hero elements
        const heroContent = document.querySelector('.hero-content');
        if (heroContent) {
            heroContent.style.transform = `translateY(${scrolled * 0.3}px)`;
            heroContent.style.opacity = 1 - (scrolled / 500);
        }
        
        const terminalWindow = document.querySelector('.terminal-window');
        if (terminalWindow) {
            terminalWindow.style.transform = `translateY(${scrolled * 0.2}px)`;
        }
    }
}

// ===================================
// Navbar Scroll Effect
// ===================================

let lastScroll = 0;
const navbar = document.querySelector('.navbar');

window.addEventListener('scroll', () => {
    const currentScroll = window.pageYOffset;
    
    if (currentScroll > 100) {
        navbar.style.background = 'rgba(10, 14, 39, 0.95)';
        navbar.style.boxShadow = '0 4px 16px rgba(0, 0, 0, 0.4)';
    } else {
        navbar.style.background = 'rgba(10, 14, 39, 0.9)';
        navbar.style.boxShadow = 'none';
    }
    
    // Hide navbar on scroll down, show on scroll up
    if (currentScroll > lastScroll && currentScroll > 500) {
        navbar.style.transform = 'translateY(-100%)';
    } else {
        navbar.style.transform = 'translateY(0)';
    }
    
    lastScroll = currentScroll;
});

// ===================================
// Performance Monitoring
// ===================================

// Log performance metrics
if (window.performance && window.performance.timing) {
    window.addEventListener('load', () => {
        setTimeout(() => {
            const perfData = window.performance.timing;
            const pageLoadTime = perfData.loadEventEnd - perfData.navigationStart;
            console.log(`⚡ Page load time: ${pageLoadTime}ms`);
        }, 0);
    });
}

// ===================================
// Easter Eggs
// ===================================

// Konami code easter egg
let konamiCode = [];
const konamiSequence = [38, 38, 40, 40, 37, 39, 37, 39, 66, 65];

document.addEventListener('keydown', (e) => {
    konamiCode.push(e.keyCode);
    konamiCode = konamiCode.slice(-10);
    
    if (konamiCode.join(',') === konamiSequence.join(',')) {
        activateMatrixMode();
    }
});

function activateMatrixMode() {
    const body = document.body;
    body.style.animation = 'matrix 5s linear infinite';
    
    const style = document.createElement('style');
    style.textContent = `
        @keyframes matrix {
            0% { filter: hue-rotate(0deg); }
            100% { filter: hue-rotate(360deg); }
        }
    `;
    document.head.appendChild(style);
    
    // Create falling characters effect
    const canvas = document.createElement('canvas');
    canvas.style.position = 'fixed';
    canvas.style.top = '0';
    canvas.style.left = '0';
    canvas.style.width = '100%';
    canvas.style.height = '100%';
    canvas.style.pointerEvents = 'none';
    canvas.style.zIndex = '9999';
    canvas.style.opacity = '0.3';
    document.body.appendChild(canvas);
    
    const ctx = canvas.getContext('2d');
    canvas.width = window.innerWidth;
    canvas.height = window.innerHeight;
    
    const chars = '01アイウエオカキクケコサシスセソタチツテト';
    const fontSize = 14;
    const columns = canvas.width / fontSize;
    const drops = Array(Math.floor(columns)).fill(1);
    
    function drawMatrix() {
        ctx.fillStyle = 'rgba(10, 14, 39, 0.05)';
        ctx.fillRect(0, 0, canvas.width, canvas.height);
        
        ctx.fillStyle = '#00d4ff';
        ctx.font = fontSize + 'px monospace';
        
        drops.forEach((y, i) => {
            const text = chars[Math.floor(Math.random() * chars.length)];
            const x = i * fontSize;
            ctx.fillText(text, x, y * fontSize);
            
            if (y * fontSize > canvas.height && Math.random() > 0.975) {
                drops[i] = 0;
            }
            drops[i]++;
        });
    }
    
    const matrixInterval = setInterval(drawMatrix, 50);
    
    // Stop after 10 seconds
    setTimeout(() => {
        clearInterval(matrixInterval);
        canvas.remove();
        style.remove();
        body.style.animation = '';
    }, 10000);
    
    console.log('🎮 Matrix mode activated! Welcome to the SecBeat.');
}

// ===================================
// Utility Functions
// ===================================

// Copy code to clipboard
document.querySelectorAll('.code-block').forEach(block => {
    const button = document.createElement('button');
    button.textContent = '📋';
    button.style.cssText = 'position: absolute; top: 8px; right: 8px; background: var(--bg-tertiary); border: 1px solid var(--border-color); padding: 4px 8px; border-radius: 4px; cursor: pointer; opacity: 0; transition: opacity 0.3s;';
    
    block.style.position = 'relative';
    block.appendChild(button);
    
    block.addEventListener('mouseenter', () => button.style.opacity = '1');
    block.addEventListener('mouseleave', () => button.style.opacity = '0');
    
    button.addEventListener('click', () => {
        const code = block.querySelector('code').textContent;
        navigator.clipboard.writeText(code).then(() => {
            button.textContent = '✓';
            setTimeout(() => button.textContent = '📋', 2000);
        });
    });
});

// ===================================
// Node Resilience Simulation
// ===================================

function initNodeResilience() {
    const nodes = document.querySelectorAll('.node-resilient');
    if (nodes.length === 0) return;
    
    const statuses = [
        { status: 'healthy', icon: '🟢', text: 'Active', duration: 5000 },
        { status: 'under-attack', icon: '🛡️', text: 'Defending', duration: 4000 },
        { status: 'recovering', icon: '🔄', text: 'Auto-healing', duration: 3000 }
    ];
    
    nodes.forEach((node, index) => {
        // Start each node with a different initial delay
        let currentStatusIndex = index % statuses.length;
        
        function updateNodeStatus() {
            const status = statuses[currentStatusIndex];
            node.setAttribute('data-status', status.status);
            
            const statusElement = node.querySelector('.component-status');
            if (statusElement) {
                statusElement.textContent = `${status.icon} ${status.text}`;
            }
            
            // Move to next status
            currentStatusIndex = (currentStatusIndex + 1) % statuses.length;
            
            // Schedule next update with status-specific duration
            setTimeout(updateNodeStatus, status.duration);
        }
        
        // Start with initial delay based on node index
        setTimeout(updateNodeStatus, index * 1500);
    });
}

// Log ASCII art on console
console.log(`
 ____            ____                _   
/ ___|  ___  ___| __ )  ___  __ _  | |_ 
\\___ \\ / _ \\/ __|  _ \\ / _ \\/ _\` | | __|
 ___) |  __/ (__| |_) |  __/ (_| | | |_ 
|____/ \\___|\\___|____/ \\___|\\__,_|  \\__|
                                        
🛡️  DDoS Mitigation & WAF Platform
⚡ High-performance Rust-based protection
🌐 https://github.com/fabriziosalmi/secbeat
`);
