import React from 'react';

interface FooterProps {}

const Footer: React.FC<FooterProps> = () => {
  return (
    <footer className="bg-gray-800 text-white p-4 mt-auto">
      <div className="container mx-auto">
        <div className="flex flex-col md:flex-row justify-between items-center">
          <div className="mb-4 md:mb-0">
            <p className="text-sm">&copy; {new Date().getFullYear()} Hosting Farm. All rights reserved.</p>
          </div>
          <div className="flex space-x-4">
            <a href="#" className="text-sm hover:text-gray-300">Terms of Service</a>
            <a href="#" className="text-sm hover:text-gray-300">Privacy Policy</a>
            <a href="#" className="text-sm hover:text-gray-300">Contact</a>
          </div>
        </div>
      </div>
    </footer>
  );
};

export default Footer;
