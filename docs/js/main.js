/**
 * Aegis Marketing Site - Main JavaScript
 * Handles mobile menu, smooth scrolling, and FAQ accordion
 */

(function() {
  'use strict';

  // Wait for DOM to be ready
  document.addEventListener('DOMContentLoaded', init);

  function init() {
    initMobileMenu();
    initFaqAccordion();
    initSmoothScroll();
    initHeaderScroll();
  }

  /**
   * Mobile Menu Toggle
   */
  function initMobileMenu() {
    const menuBtn = document.querySelector('.mobile-menu-btn');
    const mobileNav = document.querySelector('.mobile-nav');

    if (!menuBtn || !mobileNav) return;

    menuBtn.addEventListener('click', function() {
      const isExpanded = menuBtn.getAttribute('aria-expanded') === 'true';

      menuBtn.setAttribute('aria-expanded', !isExpanded);
      menuBtn.classList.toggle('active');
      mobileNav.classList.toggle('active');
    });

    // Close menu when clicking a link
    const mobileLinks = mobileNav.querySelectorAll('a');
    mobileLinks.forEach(function(link) {
      link.addEventListener('click', function() {
        menuBtn.setAttribute('aria-expanded', 'false');
        menuBtn.classList.remove('active');
        mobileNav.classList.remove('active');
      });
    });

    // Close menu when clicking outside
    document.addEventListener('click', function(e) {
      if (!menuBtn.contains(e.target) && !mobileNav.contains(e.target)) {
        menuBtn.setAttribute('aria-expanded', 'false');
        menuBtn.classList.remove('active');
        mobileNav.classList.remove('active');
      }
    });

    // Close menu on escape key
    document.addEventListener('keydown', function(e) {
      if (e.key === 'Escape' && mobileNav.classList.contains('active')) {
        menuBtn.setAttribute('aria-expanded', 'false');
        menuBtn.classList.remove('active');
        mobileNav.classList.remove('active');
        menuBtn.focus();
      }
    });
  }

  /**
   * FAQ Accordion
   */
  function initFaqAccordion() {
    const faqItems = document.querySelectorAll('.faq-item');

    faqItems.forEach(function(item) {
      const question = item.querySelector('.faq-question');
      const answer = item.querySelector('.faq-answer');

      if (!question || !answer) return;

      question.addEventListener('click', function() {
        const isExpanded = question.getAttribute('aria-expanded') === 'true';

        // Close all other items
        faqItems.forEach(function(otherItem) {
          if (otherItem !== item) {
            otherItem.classList.remove('active');
            const otherQuestion = otherItem.querySelector('.faq-question');
            if (otherQuestion) {
              otherQuestion.setAttribute('aria-expanded', 'false');
            }
          }
        });

        // Toggle current item
        item.classList.toggle('active');
        question.setAttribute('aria-expanded', !isExpanded);
      });

      // Keyboard support
      question.addEventListener('keydown', function(e) {
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          question.click();
        }
      });
    });
  }

  /**
   * Smooth Scroll for anchor links (fallback for browsers without CSS smooth scroll)
   */
  function initSmoothScroll() {
    // Check if browser supports CSS scroll-behavior
    if ('scrollBehavior' in document.documentElement.style) {
      return; // Use native smooth scrolling
    }

    const links = document.querySelectorAll('a[href^="#"]');

    links.forEach(function(link) {
      link.addEventListener('click', function(e) {
        const targetId = this.getAttribute('href');
        if (targetId === '#') return;

        const target = document.querySelector(targetId);
        if (!target) return;

        e.preventDefault();

        const headerHeight = document.querySelector('.header').offsetHeight;
        const targetPosition = target.getBoundingClientRect().top + window.pageYOffset - headerHeight;

        window.scrollTo({
          top: targetPosition,
          behavior: 'smooth'
        });

        // Update URL without scrolling
        history.pushState(null, null, targetId);
      });
    });
  }

  /**
   * Header background on scroll (optional enhancement)
   */
  function initHeaderScroll() {
    const header = document.querySelector('.header');
    if (!header) return;

    let lastScroll = 0;

    window.addEventListener('scroll', function() {
      const currentScroll = window.pageYOffset;

      // Add shadow when scrolled
      if (currentScroll > 10) {
        header.style.boxShadow = '0 2px 10px rgba(0, 0, 0, 0.3)';
      } else {
        header.style.boxShadow = 'none';
      }

      lastScroll = currentScroll;
    }, { passive: true });
  }

})();
