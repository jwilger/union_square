// Smooth scrolling for navigation links
document.querySelectorAll('a[href^="#"]').forEach(anchor => {
    anchor.addEventListener('click', function (e) {
        e.preventDefault();
        const target = document.querySelector(this.getAttribute('href'));
        if (target) {
            target.scrollIntoView({
                behavior: 'smooth',
                block: 'start'
            });
        }
    });
});

// Navbar scroll effect
let lastScroll = 0;
const navbar = document.querySelector('.navbar');

window.addEventListener('scroll', () => {
    const currentScroll = window.pageYOffset;

    if (currentScroll <= 0) {
        navbar.style.boxShadow = 'none';
    } else {
        navbar.style.boxShadow = '0 2px 20px rgba(0, 0, 0, 0.3)';
    }

    lastScroll = currentScroll;
});

// Intersection Observer for fade-in animations
const observerOptions = {
    threshold: 0.1,
    rootMargin: '0px 0px -100px 0px'
};

const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
        if (entry.isIntersecting) {
            entry.target.style.opacity = '1';
            entry.target.style.transform = 'translateY(0)';
        }
    });
}, observerOptions);

// Apply intersection observer to feature cards and step cards
document.addEventListener('DOMContentLoaded', () => {
    const elements = document.querySelectorAll('.feature-card, .step-card');
    elements.forEach((el, index) => {
        el.style.opacity = '0';
        el.style.transform = 'translateY(20px)';
        el.style.transition = `all 0.6s ease ${index * 0.1}s`;
        observer.observe(el);
    });
});

// Add syntax highlighting effect to code blocks
document.querySelectorAll('.code-block code').forEach(block => {
    // Get the text content and preserve it
    let text = block.textContent;

    // Create a document fragment to build the highlighted content
    const fragment = document.createDocumentFragment();

    // Simple syntax highlighting for Rust code using text manipulation
    const tokens = text.split(/(\s+|[(){}[\]<>,;:&|!?*+=\-/\\])/);

    const keywords = new Set(['pub', 'async', 'fn', 'struct', 'impl', 'let', 'const', 'use', 'mod', 'trait', 'enum', 'match', 'if', 'else', 'for', 'while', 'loop', 'return', 'break', 'continue', 'self', 'Self', 'super', 'crate', 'move', 'ref', 'mut', 'where', 'type', 'unsafe', 'extern', 'static', 'as', 'in', 'from', 'into']);
    const types = new Set(['String', 'Result', 'Ok', 'Err', 'Option', 'Some', 'None', 'Vec', 'HashMap', 'bool', 'u8', 'u16', 'u32', 'u64', 'i8', 'i16', 'i32', 'i64', 'f32', 'f64', 'usize', 'isize', 'char', 'str', 'SessionId', 'ProxyRequest', 'ProxyResponse', 'ProxyError']);

    let inString = false;
    let inComment = false;
    let inAttribute = false;

    for (let i = 0; i < tokens.length; i++) {
        const token = tokens[i];

        if (token === '"' && !inComment) {
            inString = !inString;
            const span = document.createElement('span');
            span.className = 'syntax-string';
            span.textContent = token;
            fragment.appendChild(span);
        } else if (token === '//' && !inString) {
            inComment = true;
            const span = document.createElement('span');
            span.className = 'syntax-comment';
            span.textContent = token;
            fragment.appendChild(span);
        } else if (token === '\n' && inComment) {
            inComment = false;
            fragment.appendChild(document.createTextNode(token));
        } else if (token.startsWith('#[') && !inString && !inComment) {
            inAttribute = true;
            const span = document.createElement('span');
            span.className = 'syntax-attribute';
            span.textContent = token;
            fragment.appendChild(span);
        } else if (token.includes(']') && inAttribute) {
            inAttribute = false;
            const span = document.createElement('span');
            span.className = 'syntax-attribute';
            span.textContent = token;
            fragment.appendChild(span);
        } else if (inString || inComment || inAttribute) {
            const span = document.createElement('span');
            span.className = inString ? 'syntax-string' : (inComment ? 'syntax-comment' : 'syntax-attribute');
            span.textContent = token;
            fragment.appendChild(span);
        } else if (keywords.has(token)) {
            const span = document.createElement('span');
            span.className = 'syntax-keyword';
            span.textContent = token;
            fragment.appendChild(span);
        } else if (types.has(token)) {
            const span = document.createElement('span');
            span.className = 'syntax-type';
            span.textContent = token;
            fragment.appendChild(span);
        } else if (/^[a-z_][a-zA-Z0-9_]*$/.test(token) && i + 1 < tokens.length && tokens[i + 1] === '(') {
            const span = document.createElement('span');
            span.className = 'syntax-function';
            span.textContent = token;
            fragment.appendChild(span);
        } else {
            fragment.appendChild(document.createTextNode(token));
        }
    }

    // Clear the block and append the highlighted content
    block.textContent = '';
    block.appendChild(fragment);
});

// Active navigation link highlighting
const sections = document.querySelectorAll('section[id]');
const navLinks = document.querySelectorAll('.nav-link');

window.addEventListener('scroll', () => {
    let current = '';

    sections.forEach(section => {
        const sectionTop = section.offsetTop;
        const sectionHeight = section.clientHeight;
        if (pageYOffset >= sectionTop - 200) {
            current = section.getAttribute('id');
        }
    });

    navLinks.forEach(link => {
        link.classList.remove('active');
        if (link.getAttribute('href') === `#${current}`) {
            link.classList.add('active');
        }
    });
});

// Add CSS for active state
const style = document.createElement('style');
style.textContent = `
    .nav-link.active {
        color: var(--accent-primary);
    }
    .nav-link.active::after {
        width: 100%;
    }
`;
document.head.appendChild(style);

// Fetch and display latest GitHub release
async function fetchLatestRelease() {
    try {
        const response = await fetch('https://api.github.com/repos/jwilger/union_square/releases/latest');

        if (response.status === 404) {
            // No releases yet
            return;
        }

        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }

        const release = await response.json();

        // Update release card with actual release data
        const releaseCard = document.querySelector('.release-card');
        if (!releaseCard) return;

        const releaseDate = new Date(release.published_at).toLocaleDateString('en-US', {
            year: 'numeric',
            month: 'long',
            day: 'numeric'
        });

        releaseCard.innerHTML = `
            <div class="release-status">
                <span class="status-badge">${release.prerelease ? 'Pre-release' : 'Latest'}</span>
                <span class="release-date">${releaseDate}</span>
            </div>
            <h3 class="release-title">${release.name || release.tag_name}</h3>
            <div class="release-description">${marked.parse(release.body || 'No release notes available.')}</div>
            <div class="release-actions">
                <a href="${release.html_url}" class="btn btn-outline">View Release</a>
                <a href="https://github.com/jwilger/union_square/releases" class="btn btn-outline">All Releases</a>
            </div>
        `;
    } catch (error) {
        console.error('Error fetching release:', error);
        // Keep the default content on error
    }
}

// Load marked.js for markdown parsing
const script = document.createElement('script');
script.src = 'https://cdn.jsdelivr.net/npm/marked/marked.min.js';
script.onload = () => {
    // Fetch release once marked.js is loaded
    fetchLatestRelease();
};
document.head.appendChild(script);
