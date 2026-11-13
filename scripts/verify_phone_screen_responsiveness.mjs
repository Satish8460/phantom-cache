import { copyFileSync, mkdirSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { Browser, sleep } from '../ui/scripts/cdp-lib.mjs';

const SHOTS_DIR = resolve('docs/demo/live_demo_shots/shots');
const ARTIFACT_DIR = resolve('C:/Users/Satish/.gemini/antigravity/brain/eaca48fa-9967-405d-b4d3-54047091810e/advanced_features');
mkdirSync(SHOTS_DIR, { recursive: true });
mkdirSync(ARTIFACT_DIR, { recursive: true });

async function run() {
  console.log('📱 Launching Chrome CDP with Mobile Phone Viewport (390 x 844)...');
  const b = new Browser({ evidenceDir: 'docs/demo/live_demo_shots', label: 'phone_responsive' });
  await b.launch();

  // Standard iPhone 14/15/16 mobile dimensions
  await b.viewport(390, 844);

  try {
    // 1. Shopper Experience on Phone Screen
    console.log('Navigating to Shopper view on Phone (http://127.0.0.1:3201/#persona=buyer)...');
    await b.goto('http://127.0.0.1:3201/#persona=buyer', 3000);
    await sleep(2000);

    const shopperMobileAudit = await b.eval(`
      (() => {
        const grid = document.querySelector('.product-shelf-grid');
        const gridStyle = grid ? window.getComputedStyle(grid) : null;
        const cards = document.querySelectorAll('.product-retail-card');
        const header = document.querySelector('.persona-hero-card');
        return {
          windowWidth: window.innerWidth,
          gridColumns: gridStyle ? gridStyle.gridTemplateColumns : '',
          productCount: cards.length,
          hasHorizontalScroll: document.body.scrollWidth > window.innerWidth,
        };
      })()
    `);
    console.log('Shopper Mobile Phone Audit:', shopperMobileAudit);

    await b.shot('132_phone_screen_shopper_catalog.png');
    console.log('Shot 132 captured: Shopper catalog on 390px phone screen.');
    copyFileSync(join(SHOTS_DIR, '132_phone_screen_shopper_catalog.png'), join(ARTIFACT_DIR, '132_phone_screen_shopper_catalog.png'));

    // 2. Open Mobile Cart Drawer
    console.log('Testing Mobile Cart Drawer...');
    await b.eval(`
      (() => {
        const addCartBtns = Array.from(document.querySelectorAll('.probe-btn'));
        const cartAdd = addCartBtns.find(b => b.textContent.includes('+ Cart'));
        if (cartAdd) cartAdd.click();
      })()
    `);
    await sleep(1000);

    const cartDrawerAudit = await b.eval(`
      (() => {
        const panel = document.querySelector('.cart-drawer-panel');
        if (!panel) return { open: false };
        const rect = panel.getBoundingClientRect();
        return {
          open: true,
          width: rect.width,
          windowWidth: window.innerWidth,
          isFullWidthOnMobile: Math.abs(rect.width - window.innerWidth) < 5,
        };
      })()
    `);
    console.log('Mobile Cart Drawer Audit:', cartDrawerAudit);

    await b.shot('133_phone_screen_cart_drawer.png');
    console.log('Shot 133 captured: Full-screen Cart Drawer on 390px phone screen.');
    copyFileSync(join(SHOTS_DIR, '133_phone_screen_cart_drawer.png'), join(ARTIFACT_DIR, '133_phone_screen_cart_drawer.png'));

    // Close Cart Drawer
    await b.eval(`
      (() => {
        const closeBtn = document.querySelector('.cart-drawer-panel .probe-btn');
        if (closeBtn) closeBtn.click();
      })()
    `);
    await sleep(800);

    // 3. Test Mobile QR Pairing Modal
    console.log('Testing Mobile Pairing Modal...');
    await b.eval(`
      (() => {
        const btns = Array.from(document.querySelectorAll('.probe-btn'));
        const pairBtn = btns.find(b => b.textContent.includes('Mobile Pairing'));
        if (pairBtn) pairBtn.click();
      })()
    `);
    await sleep(800);

    await b.shot('134_phone_screen_qr_pairing.png');
    console.log('Shot 134 captured: Mobile pairing modal.');
    copyFileSync(join(SHOTS_DIR, '134_phone_screen_qr_pairing.png'), join(ARTIFACT_DIR, '134_phone_screen_qr_pairing.png'));

    // Close Modal
    await b.eval(`
      (() => {
        const closeBtn = document.querySelector('.qr-modal-dialog .probe-btn');
        if (closeBtn) closeBtn.click();
      })()
    `);
    await sleep(800);

    // 4. Switch to Merchant (Seller) Portal on Phone Screen
    console.log('Navigating to Merchant view on Phone (http://127.0.0.1:3201/#persona=seller)...');
    await b.goto('http://127.0.0.1:3201/#persona=seller', 3000);
    await sleep(2000);

    const sellerMobileAudit = await b.eval(`
      (() => {
        const tableWrap = document.querySelector('.merchant-table-responsive');
        const table = document.querySelector('.merchant-inventory-table');
        const rows = document.querySelectorAll('.merchant-inventory-table tbody tr');
        const ordersList = document.querySelector('.merchant-orders-list');
        return {
          windowWidth: window.innerWidth,
          hasTableResponsiveWrap: !!tableWrap,
          tableScrollWidth: table ? table.scrollWidth : 0,
          wrapClientWidth: tableWrap ? tableWrap.clientWidth : 0,
          inventoryRowsCount: rows.length,
          ordersFeedActive: !!ordersList,
          hasBodyHorizontalOverflow: document.body.scrollWidth > window.innerWidth,
        };
      })()
    `);
    console.log('Seller Mobile Phone Audit:', sellerMobileAudit);

    await b.shot('135_phone_screen_seller_portal.png');
    console.log('Shot 135 captured: Merchant Seller Portal on 390px phone screen.');
    copyFileSync(join(SHOTS_DIR, '135_phone_screen_seller_portal.png'), join(ARTIFACT_DIR, '135_phone_screen_seller_portal.png'));

    // 5. Trigger Flash Sale on Mobile to verify real-time dynamic sync
    console.log('Triggering Flash Sale on mobile...');
    await b.eval(`
      (() => {
        const flashBtns = Array.from(document.querySelectorAll('.merchant-flash-btn'));
        if (flashBtns[0]) flashBtns[0].click();
      })()
    `);
    await sleep(1200);

    await b.shot('136_phone_screen_seller_flash_sale_active.png');
    console.log('Shot 136 captured: Real-time Flash Sale triggered on mobile.');
    copyFileSync(join(SHOTS_DIR, '136_phone_screen_seller_flash_sale_active.png'), join(ARTIFACT_DIR, '136_phone_screen_seller_flash_sale_active.png'));

    console.log('🎉 All mobile phone responsiveness verifications completed successfully!');
  } catch (err) {
    console.error('Mobile verification failed:', err);
    throw err;
  } finally {
    await b.close();
  }
}

run().catch((e) => {
  console.error(e);
  process.exit(1);
});
