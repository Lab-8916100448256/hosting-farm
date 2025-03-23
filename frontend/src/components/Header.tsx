import React from 'react';
import { Link } from 'react-router-dom';

interface HeaderProps {
  isAuthenticated: boolean;
  userName?: string;
  onLogout: () => void;
}

const Header: React.FC<HeaderProps> = ({ isAuthenticated, userName, onLogout }) => {
  return (
    <header className="bg-gray-800 text-white p-4">
      <div className="container mx-auto flex justify-between items-center">
        <div className="flex items-center">
          <Link to="/" className="text-xl font-bold">Hosting Farm</Link>
        </div>
        
        <nav>
          <ul className="flex space-x-6">
            {isAuthenticated ? (
              <>
                <li>
                  <Link to="/teams" className="hover:text-gray-300">Teams</Link>
                </li>
                <li>
                  <Link to="/users/profile" className="hover:text-gray-300">Profile</Link>
                </li>
                <li>
                  <Link to="/users/invitations" className="hover:text-gray-300">Invitations</Link>
                </li>
                <li className="flex items-center">
                  <span className="mr-4">Welcome, {userName}</span>
                  <button 
                    onClick={onLogout}
                    className="bg-red-600 hover:bg-red-700 px-3 py-1 rounded"
                  >
                    Logout
                  </button>
                </li>
              </>
            ) : (
              <>
                <li>
                  <Link to="/auth/login" className="hover:text-gray-300">Login</Link>
                </li>
                <li>
                  <Link to="/auth/register" className="hover:text-gray-300">Register</Link>
                </li>
              </>
            )}
          </ul>
        </nav>
      </div>
    </header>
  );
};

export default Header;
