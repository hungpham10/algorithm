import 'dotenv/config';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';
import mime from 'mime';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const DIST_DIR = path.join(__dirname, '../dist');

/**
 * Hàm lấy Access Token từ Auth0 (M2M)
 */
async function getAuth0Token() {
  console.log("🔑 Đang lấy token từ Auth0...");
  const response = await fetch(process.env.AUTH0_URL, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      client_id: process.env.CLIENT_ID,
      client_secret: process.env.CLIENT_SECRET,
      audience: process.env.AUDIENCE,
      grant_type: 'client_credentials'
    })
  });

  if (!response.ok) {
    const error = await response.text();
    throw new Error(`Không thể lấy token: ${error}`);
  }

  const data = await response.json();
  return data.access_token;
}

/**
 * Hàm lấy toàn bộ danh sách file
 */
function getAllFiles(dirPath, arrayOfFiles = []) {
  const files = fs.readdirSync(dirPath);
  files.forEach((file) => {
    const fullPath = path.join(dirPath, file);
    if (fs.statSync(fullPath).isDirectory()) {
      getAllFiles(fullPath, arrayOfFiles);
    } else {
      arrayOfFiles.push(fullPath);
    }
  });
  return arrayOfFiles;
}

/**
 * Main function
 */
async function uploadAll() {
  try {
    // 1. Lấy token trước
    const accessToken = await getAuth0Token();
    console.log("✅ Lấy token thành công.");

    if (!fs.existsSync(DIST_DIR)) {
      console.error("❌ Thư mục dist không tồn tại. Chạy 'npm run build' trước.");
      return;
    }

    const allFiles = getAllFiles(DIST_DIR);
    console.log(`🚀 Bắt đầu upload ${allFiles.length} files...`);

    for (const filePath of allFiles) {
      const relativePath = path.relative(DIST_DIR, filePath).replace(/\\/g, '/');
      const fileBuffer = fs.readFileSync(filePath);
      const contentType = mime.getType(filePath) || 'application/octet-stream';

      const formData = new FormData();
      const blob = new Blob([fileBuffer], { type: contentType });
      formData.append('file', blob, relativePath);

      // Điểm thay đổi: Kiểm tra nếu là file .html thì đổi API Endpoint
      const isHtml = path.extname(filePath).toLowerCase() === '.html';
      const uploadUrl = isHtml ? process.env.API_PUBLISH_SITE : process.env.API_PUBLISH_FILE;

      const response = await fetch(uploadUrl, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${accessToken}`,
        },
        body: formData
      });

      if (response.ok) {
        console.log(`✅ Uploaded (${isHtml ? 'HTML' : 'Asset'}): ${relativePath}`);
      } else {
        console.error(`❌ Failed: ${relativePath} (${response.status}): ${await response.text()}`);
      }
    }

    // Gọi API Purge Cache sau khi hoàn tất
    if (process.env.API_PURGE_URL) {
      await fetch(`${process.env.API_PURGE_URL}`, {
        method: 'HEAD',
        headers: {
          'Authorization': `Bearer ${accessToken}`,
        },
      });
    }

    console.log("\n✨ Publish hoàn tất!");
  } catch (error) {
    console.error(`💥 Lỗi hệ thống: ${error.message}`);
  }
}

uploadAll();
