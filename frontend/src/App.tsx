import React from 'react';
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
import Header from './components/Header';
import Footer from './components/Footer';
import Login from './pages/auth/Login';
import Register from './pages/auth/Register';
import TeamsList from './pages/teams/TeamsList';
import TeamDetails from './pages/teams/TeamDetails';
import CreateTeam from './pages/teams/CreateTeam';
import UserProfile from './pages/users/UserProfile';
import UserInvitations from './pages/users/UserInvitations';

const App: React.FC = () => {
  const [isAuthenticated, setIsAuthenticated] = React.useState<boolean>(false);
  const [userName, setUserName] = React.useState<string>('');
  
  React.useEffect(() => {
    // Check if user is authenticated
    const token = localStorage.getItem('token');
    if (token) {
      // TODO: Validate token with backend
      setIsAuthenticated(true);
      const storedName = localStorage.getItem('userName');
      if (storedName) {
        setUserName(storedName);
      }
    }
  }, []);
  
  const handleLogout = () => {
    localStorage.removeItem('token');
    localStorage.removeItem('userName');
    setIsAuthenticated(false);
    setUserName('');
  };
  
  return (
    <Router>
      <div className="flex flex-col min-h-screen">
        <Header 
          isAuthenticated={isAuthenticated} 
          userName={userName} 
          onLogout={handleLogout} 
        />
        
        <main className="container mx-auto p-4 flex-grow">
          <Routes>
            {/* Public routes */}
            <Route path="/auth/login" element={
              !isAuthenticated ? <Login setIsAuthenticated={setIsAuthenticated} setUserName={setUserName} /> : <Navigate to="/teams" />
            } />
            <Route path="/auth/register" element={
              !isAuthenticated ? <Register setIsAuthenticated={setIsAuthenticated} setUserName={setUserName} /> : <Navigate to="/teams" />
            } />
            
            {/* Protected routes */}
            <Route path="/teams" element={
              isAuthenticated ? <TeamsList /> : <Navigate to="/auth/login" />
            } />
            <Route path="/teams/new" element={
              isAuthenticated ? <CreateTeam /> : <Navigate to="/auth/login" />
            } />
            <Route path="/teams/:teamId" element={
              isAuthenticated ? <TeamDetails /> : <Navigate to="/auth/login" />
            } />
            <Route path="/users/profile" element={
              isAuthenticated ? <UserProfile /> : <Navigate to="/auth/login" />
            } />
            <Route path="/users/invitations" element={
              isAuthenticated ? <UserInvitations /> : <Navigate to="/auth/login" />
            } />
            
            {/* Default route */}
            <Route path="/" element={
              isAuthenticated ? <Navigate to="/teams" /> : <Navigate to="/auth/login" />
            } />
          </Routes>
        </main>
        
        <Footer />
      </div>
    </Router>
  );
};

export default App;
