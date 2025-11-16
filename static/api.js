/**
 * API utility for consistent fetch handling
 */

const API_BASE = window.location.origin;

/**
 * Make an API request
 * @param {string} endpoint - API endpoint (e.g., '/posts')
 * @param {Object} options - Fetch options
 * @param {string} options.method - HTTP method (default: 'GET')
 * @param {Object} options.body - Request body (auto-stringified)
 * @param {string} options.token - Auth token (auto-added as Bearer)
 * @param {boolean} options.json - Parse response as JSON (default: true)
 * @returns {Promise<{status: number, data: any, ok: boolean}>}
 */
async function apiCall(endpoint, options = {}) {
    const {
        method = 'GET',
        body = null,
        token = null,
        json = true
    } = options;

    const headers = {
        'Content-Type': 'application/json'
    };

    if (token) {
        headers['Authorization'] = `Bearer ${token}`;
    }

    const fetchOptions = {
        method,
        headers
    };

    if (body) {
        fetchOptions.body = JSON.stringify(body);
    }

    try {
        const res = await fetch(API_BASE + endpoint, fetchOptions);
        let data = null;

        if (json) {
            try {
                data = await res.json();
            } catch (err) {
                data = null;
            }
        } else {
            data = await res.text();
        }

        return {
            status: res.status,
            ok: res.ok,
            data
        };
    } catch (err) {
        return {
            status: 0,
            ok: false,
            data: null,
            error: err.message
        };
    }
}

/**
 * Show error message (requires showError function in global scope)
 */
function handleError(error, statusCode) {
    const errorMessages = {
        0: 'Network error: ' + error,
        400: 'Invalid request',
        401: 'Unauthorized - please log in',
        403: 'Access denied',
        404: 'Not found',
        409: 'Conflict - resource already exists',
        500: 'Server error'
    };

    const msg = errorMessages[statusCode] || errorMessages[0] || 'Error';
    if (typeof showError === 'function') {
        showError(msg);
    }
}

/**
 * Show success message (requires showSuccess function in global scope)
 */
function handleSuccess(message) {
    if (typeof showSuccess === 'function') {
        showSuccess(message);
    }
}

/**
 * Render posts to a container
 * @param {Array} postsArray - Array of post objects
 * @param {string} containerId - ID of the container element
 * @param {boolean} showUsername - Whether to show username link (default: true)
 * @param {boolean} showActionsForOwnOnly - Whether to show edit/delete buttons only for own posts (default: false)
 * @param {string} currentUserId - Current user ID (required if showActionsForOwnOnly is true)
 */
function renderPosts(postsArray, containerId, showUsername = true, showActionsForOwnOnly = false, currentUserId = null) {
    const container = document.getElementById(containerId);
    
    if (postsArray.length === 0) {
        container.innerHTML = '<p style="color: #999; text-align: center;">No posts yet</p>';
        return;
    }
    
    container.innerHTML = postsArray.map(p => `
        <div class="post">
            ${showUsername ? `<div style="font-size: 13px; color: #666; margin-bottom: 8px; font-weight: 500;">
                <a href="/${p.username}" style="color: #209CEE; text-decoration: none;">${p.username}</a>
            </div>` : ''}
            <div class="post-content">${p.content}</div>
            <div class="post-meta">
                <div>
                    <span>${new Date(p.created_at).toLocaleString()}</span>
                    ${p.updated_at ? `<span class="edited-badge" title="Updated: ${new Date(p.updated_at).toLocaleString()}">(edited)</span>` : ''}
                </div>
                ${showActionsForOwnOnly && currentUserId && p.user_id === currentUserId ? `<div class="post-actions">
                    <button class="edit-btn" data-post-id="${p.id}" title="Edit Post">E</button>
                    <button class="delete-btn" onclick="deletePost('${p.id}')" title="Delete Post">X</button>
                </div>` : ''}
            </div>
        </div>
    `).join('');
}
