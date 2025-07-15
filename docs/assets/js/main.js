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
    // Simple syntax highlighting for Rust code
    let html = block.innerHTML;
    
    // Keywords
    html = html.replace(/\b(pub|async|fn|struct|impl|let|const|use|mod|trait|enum|match|if|else|for|while|loop|return|break|continue|self|Self|super|crate|move|ref|mut|where|type|unsafe|extern|static|as|in|from|into)\b/g, '<span style="color: var(--ctp-mauve);">$1</span>');
    
    // Types
    html = html.replace(/\b(String|Result|Ok|Err|Option|Some|None|Vec|HashMap|bool|u8|u16|u32|u64|i8|i16|i32|i64|f32|f64|usize|isize|char|str)\b/g, '<span style="color: var(--ctp-yellow);">$1</span>');
    
    // Functions
    html = html.replace(/\b([a-z_][a-zA-Z0-9_]*)\s*\(/g, '<span style="color: var(--ctp-blue);">$1</span>(');
    
    // Strings
    html = html.replace(/"([^"]*)"/g, '<span style="color: var(--ctp-green);">"$1"</span>');
    
    // Comments
    html = html.replace(/(\/\/[^\n]*)/g, '<span style="color: var(--ctp-overlay0);">$1</span>');
    
    // Attributes
    html = html.replace(/#\[([^\]]+)\]/g, '<span style="color: var(--ctp-peach);">#[$1]</span>');
    
    // Generics
    html = html.replace(/&lt;([^&]+)&gt;/g, '<span style="color: var(--ctp-teal);">&lt;$1&gt;</span>');
    
    block.innerHTML = html;
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