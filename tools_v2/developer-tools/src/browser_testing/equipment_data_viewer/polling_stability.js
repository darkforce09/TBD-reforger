(async () => {
    const originalFetch = window.fetch;
    const main = document.querySelector('.dv-main');
    const content = main.firstElementChild;
    const bounds = main.getBoundingClientRect();
    const input = main.querySelector('input');
    if (input) {
        input.focus();
        input.value = 'unsent search text';
        input.dispatchEvent(new Event('input', {bubbles: true}));
    }
    const outcome = { polls: 0, failure_observed: false, recovered: false,
        max_shift: 0, content_preserved: true, loading_flashes: 0,
        slow_request_completed: false, focus_preserved: true };
    let completed = 0;
    window.fetch = async function (...args) {
        const url = typeof args[0] === 'string' ? args[0] : args[0].url;
        if (!url.includes('/debug/equipment-data/status?poll=')) {
            return originalFetch.apply(this, args);
        }
        const ordinal = ++outcome.polls;
        await new Promise(resolve => setTimeout(resolve, ordinal === 1 ? 6000 : 300));
        if (ordinal === 2) {
            outcome.failure_observed = true;
            completed++;
            return new Response(JSON.stringify({error: 'Simulated status outage'}), {
                status: 500, headers: {'Content-Type': 'application/json'}
            });
        }
        const response = await originalFetch.apply(this, args);
        if (ordinal === 1 && response.ok) outcome.slow_request_completed = true;
        if (ordinal > 2 && response.ok) outcome.recovered = true;
        completed++;
        return response;
    };
    const sample = () => {
        const current = main.getBoundingClientRect();
        outcome.max_shift = Math.max(outcome.max_shift,
            Math.abs(bounds.top - current.top), Math.abs(bounds.height - current.height));
        outcome.content_preserved &&= content.isConnected && main.firstElementChild === content;
        if (input) outcome.focus_preserved &&= document.activeElement === input && input.value === 'unsent search text';
        const feedback = document.querySelector('.dv-app > .dv-feedback');
        if (feedback?.textContent.includes('Loading')) outcome.loading_flashes++;
    };
    const monitor = setInterval(sample, 20);
    try {
        const deadline = performance.now() + 22000;
        while (completed < 3 && performance.now() < deadline) {
            await new Promise(resolve => setTimeout(resolve, 50));
        }
        await new Promise(resolve => setTimeout(resolve, 100));
        sample();
        return outcome;
    } finally {
        clearInterval(monitor);
        window.fetch = originalFetch;
    }
})()
